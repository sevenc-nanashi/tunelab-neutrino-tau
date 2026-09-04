using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using TuneLab.Foundation;
using TuneLab.SDK;

namespace NeutrinoTau;

internal sealed class NeutrinoTauSynthesisSession : IVoiceSynthesisSession
{
    public unsafe NeutrinoTauSynthesisSession(
      IVoiceSynthesisContext context,
      Native.CEngine* nativeEngine)
    {
        _context = context;
        _voiceId = context.VoiceId;
        _nativeEngine = (nint)nativeEngine;

        _noteChanged = context.Notes.WhenAnyItem(
          note => note.StartTime.Modified,
          note => note.EndTime.Modified,
          note => note.Pitch.Modified,
          note => note.Lyric.Modified,
          note => note.LeadingPhonemes.Modified,
          note => note.BodyPhonemes.Modified,
          note => note.BodyOffset.Modified,
          note => note.Properties.Modified
        );
        _noteChanged.Subscribe(OnNoteChanged, _subscriptions);
        context.Notes.ItemAdded.Subscribe(OnNotesChanged, _subscriptions);
        context.Notes.ItemRemoved.Subscribe(OnNotesChanged, _subscriptions);
        context.PartProperties.Modified.Subscribe(MarkDirty, _subscriptions);
        context.Pitch.RangeModified.Subscribe(OnRangeModified, _subscriptions);
        context.PitchDeviation.RangeModified.Subscribe(OnRangeModified, _subscriptions);

        _dirty = context.Notes.Count > 0;
    }

    public string DefaultLyric => "a";

    public SynthesizedPitch SynthesizedPitch => _synthesizedPitch;
    public IReadOnlyMap<string, SynthesizedParameter> SynthesizedParameters => EmptySynthesizedParameters;
    public IReadOnlyMap<string, SynthesizedSyllable> SynthesizedPhonemes => _synthesizedPhonemes;

    public IReadOnlyList<SynthesisStatusSegment> Status
    {
        get
        {
            if (!TryGetBounds(out var startTime, out var endTime))
            {
                return [];
            }

            var status = _failed
              ? SynthesisSegmentStatus.Failed
              : _synthesizing
                ? SynthesisSegmentStatus.Synthesizing
                : _dirty || !_hasResult
                  ? SynthesisSegmentStatus.Pending
                  : SynthesisSegmentStatus.Synthesized;
            return
            [
              new SynthesisStatusSegment
        {
          StartTime = startTime,
          EndTime = endTime,
          Status = status,
          Message = _failed ? _error : null,
          Progress = _synthesizing ? _progress : 0,
        },
      ];
        }
    }

    public IActionEvent SynthesizedPhonemesChanged => _synthesizedPhonemesChanged;
    public IActionEvent SynthesizedParametersChanged => ActionEvent.Empty;
    public IActionEvent SynthesizedPitchChanged => _synthesizedPitchChanged;
    public IActionEvent StatusChanged => _statusChanged;

    public bool IsContinuation(IVoiceSynthesisNote note) => false;

    public SynthesisRange? GetNextPendingSynthesisRange(double startTime, double endTime)
    {
        if (_disposed || _synthesizing || _failed || !_dirty)
        {
            return null;
        }
        if (!TryGetBounds(out var synthesisStart, out var synthesisEnd))
        {
            return null;
        }
        if (synthesisEnd < startTime || synthesisStart > endTime)
        {
            return null;
        }
        return new SynthesisRange(synthesisStart, synthesisEnd);
    }

    public async Task SynthesizeNext(
      double startTime,
      double endTime,
      CancellationToken cancellation = default)
    {
        if (GetNextPendingSynthesisRange(startTime, endTime) == null)
        {
            return;
        }

        var notes = _context.Notes.ToList();
        var snapshot = _context.GetSnapshot(notes);
        var nativeCancelTokenAddress = CreateNativeCancelToken();
        if (nativeCancelTokenAddress == 0)
        {
            SetFailed("Failed to create a native cancellation token.");
            return;
        }

        lock (_cancelTokenLock)
        {
            _activeCancelToken = nativeCancelTokenAddress;
        }

        _dirty = false;
        _failed = false;
        _error = null;
        _synthesizing = true;
        _progress = 0;
        _statusChanged.Invoke();

        using var registration = cancellation.Register(
          () => CancelNativeToken(nativeCancelTokenAddress)
        );

        try
        {
            var response = await Task.Run(
              () => RunSynthesis(snapshot, nativeCancelTokenAddress),
              CancellationToken.None
            );

            if (_disposed || cancellation.IsCancellationRequested)
            {
                _dirty = !_disposed && _context.Notes.Count > 0;
                return;
            }
            if (_dirty)
            {
                return;
            }

            Publish(response, snapshot);
            _hasResult = true;
            _progress = 1;
        }
        catch (Exception ex)
        {
            if (cancellation.IsCancellationRequested)
            {
                _dirty = !_disposed && _context.Notes.Count > 0;
            }
            else
            {
                SetFailed($"Native synthesis failed: {ex.Message}");
            }
        }
        finally
        {
            lock (_cancelTokenLock)
            {
                if (_activeCancelToken == nativeCancelTokenAddress)
                {
                    _activeCancelToken = 0;
                }
            }
            DestroyNativeToken(nativeCancelTokenAddress);
            _synthesizing = false;
            if (!_disposed)
            {
                _statusChanged.Invoke();
            }
        }
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _subscriptions.DisposeAll();
        lock (_cancelTokenLock)
        {
            if (_activeCancelToken != 0)
            {
                CancelNativeToken(_activeCancelToken);
            }
        }
        _audioSegment?.Dispose();
        _audioSegment = null;
    }

    private unsafe SynthesisResponse RunSynthesis(
      VoiceSynthesisSnapshot snapshot,
      nint nativeCancelTokenAddress)
    {
        if (_nativeEngine == 0)
        {
            throw new InvalidOperationException("Native engine is not initialized.");
        }

        var payload = BuildPayload(snapshot);
        var payloadJson = JsonSerializer.Serialize(payload, JsonOptions);
        var payloadBytes = Encoding.UTF8.GetBytes(payloadJson + "\0");
        byte* errorPtr = null;
        byte* resultPtr = null;

        try
        {
            fixed (byte* payloadPtr = payloadBytes)
            {
                resultPtr = Native.NativeMethods.neutrino_tau_synthesize(
                  (Native.CEngine*)_nativeEngine,
                  payloadPtr,
                  (Native.CancelToken*)nativeCancelTokenAddress,
                  &errorPtr
                );
            }

            if (resultPtr == null)
            {
                var error = errorPtr != null
                  ? Marshal.PtrToStringUTF8((IntPtr)errorPtr)
                  : "Unknown native error";
                throw new InvalidOperationException(error);
            }

            var resultJson = Marshal.PtrToStringUTF8((IntPtr)resultPtr);
            if (resultJson == null)
            {
                throw new InvalidOperationException("Native synthesis returned an invalid UTF-8 response.");
            }

            var response = JsonSerializer.Deserialize<SynthesisResponse>(resultJson, JsonOptions);
            if (response == null)
            {
                throw new JsonException("Failed to parse the native synthesis response.");
            }
            return response;
        }
        finally
        {
            if (resultPtr != null)
            {
                Native.NativeMethods.neutrino_tau_free_c_string(resultPtr);
            }
            if (errorPtr != null)
            {
                Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
            }
        }
    }

    private SynthesisTaskPayload BuildPayload(VoiceSynthesisSnapshot snapshot)
    {
        if (snapshot.Notes.Count == 0)
        {
            throw new InvalidOperationException("Cannot synthesize an empty note list.");
        }

        var startTime = snapshot.Notes.Min(note => note.StartTime);
        var endTime = snapshot.Notes.Max(note => note.EndTime);
        var phonemeTimings = BuildPhonemeTimings(snapshot.Notes);
        var notes = new List<SynthesisNotePayload>(snapshot.Notes.Count);
        for (var noteIndex = 0; noteIndex < snapshot.Notes.Count; noteIndex++)
        {
            var note = snapshot.Notes[noteIndex];
            var allPhonemes = note.LeadingPhonemes.Concat(note.BodyPhonemes).ToList();
            var notePhonemes = new List<SynthesisPhonemePayload>(allPhonemes.Count);
            for (var phonemeIndex = 0; phonemeIndex < allPhonemes.Count; phonemeIndex++)
            {
                var phoneme = allPhonemes[phonemeIndex];
                var timing = phonemeTimings[noteIndex][phonemeIndex];
                notePhonemes.Add(new SynthesisPhonemePayload
                {
                    Symbol = phoneme.Symbol,
                    StartTime = timing.Start,
                    EndTime = timing.End,
                });
            }

            notes.Add(new SynthesisNotePayload
            {
                StartTime = note.StartTime,
                EndTime = note.EndTime,
                Pitch = note.Pitch,
                Lyric = note.Lyric,
                LastIndex = noteIndex > 0 ? noteIndex - 1 : null,
                NextIndex = noteIndex + 1 < snapshot.Notes.Count ? noteIndex + 1 : null,
                Properties = ConvertPropertyObject(note.Properties),
                Phonemes = notePhonemes,
            });
        }

        var pitchTimes = CollectPitchTimes(startTime, endTime);
        var pitchValues = new double[pitchTimes.Count];
        snapshot.Pitch.Evaluator.Evaluate(pitchTimes, pitchValues);
        var pitchDeviation = new double[pitchTimes.Count];
        snapshot.PitchDeviation.Evaluator.Evaluate(pitchTimes, pitchDeviation);
        for (var i = 0; i < pitchValues.Length; i++)
        {
            if (double.IsFinite(pitchValues[i]))
            {
                pitchValues[i] += pitchDeviation[i];
            }
        }

        return new SynthesisTaskPayload
        {
            VoiceId = _voiceId,
            StartTime = startTime,
            EndTime = endTime,
            Duration = Math.Max(0, endTime - startTime),
            StyleShift = ResolveNumericPartProperty(snapshot.PartProperties, "styleShift"),
            WaveformStyleShift = ResolveNumericPartProperty(snapshot.PartProperties, "waveformStyleShift"),
            PitchShiftCents = ResolveNumericPartProperty(
            snapshot.PartProperties,
            "pitchShiftCents",
            roundToInteger: false
          ),
            PartProperties = ConvertPropertyObject(snapshot.PartProperties),
            Notes = notes,
            Pitch = new PitchPayload
            {
                Times = pitchTimes,
                Values = SanitizePitchValues(pitchValues),
            },
        };
    }

    private static PhonemeTiming[][] BuildPhonemeTimings(
      IReadOnlyList<VoiceSynthesisNoteSnapshot> notes)
    {
        var layoutNotes = new PhonemeLayoutNote[notes.Count];
        for (var i = 0; i < notes.Count; i++)
        {
            var note = notes[i];
            layoutNotes[i] = new PhonemeLayoutNote
            {
                FillStart = note.StartTime,
                FillEnd = note.EndTime,
                LeadingPhonemes = note.LeadingPhonemes.Select(ToSynthesizedPhoneme).ToList(),
                BodyPhonemes = note.BodyPhonemes.Select(ToSynthesizedPhoneme).ToList(),
                BodyOffset = note.BodyOffset,
            };
        }
        return PhonemeLayout.Resolve(layoutNotes);
    }

    private static SynthesizedPhoneme ToSynthesizedPhoneme(
      VoiceSynthesisPhonemeSnapshot phoneme) => new()
      {
          Symbol = phoneme.Symbol,
          Duration = phoneme.Duration,
          StretchWeight = phoneme.StretchWeight,
      };

    private void Publish(SynthesisResponse response, VoiceSynthesisSnapshot snapshot)
    {
        if (response.Samples.Length != response.SampleCount)
        {
            throw new InvalidOperationException("Native synthesis returned an invalid sample count.");
        }

        _audioSegment?.Dispose();
        _audioSegment = _context.CreateAudioSegment(
          (long)(response.StartTime * response.SampleRate),
          response.Samples.Length,
          response.SampleRate
        );
        _audioSegment.Write(0, response.Samples);
        _audioSegment.Commit();

        _synthesizedPitch = new SynthesizedPitch
        {
            Segments = BuildSynthesizedPitch(response.PitchTimes, response.PitchValues),
        };
        _synthesizedPhonemes = BuildSynthesizedPhonemes(snapshot.Notes, response.NotePhonemes);
        _synthesizedPitchChanged.Invoke();
        _synthesizedPhonemesChanged.Invoke();
    }

    private void MarkDirty()
    {
        if (_disposed)
        {
            return;
        }

        _dirty = _context.Notes.Count > 0;
        _failed = false;
        _error = null;
        _hasResult = false;

        if (_synthesizedPitch.Segments.Count > 0)
        {
            _synthesizedPitch = EmptySynthesizedPitch;
            _synthesizedPitchChanged.Invoke();
        }
        if (_synthesizedPhonemes.Count > 0)
        {
            _synthesizedPhonemes = Map<string, SynthesizedSyllable>.Empty;
            _synthesizedPhonemesChanged.Invoke();
        }
        if (!_dirty)
        {
            _audioSegment?.Dispose();
            _audioSegment = null;
        }
        _statusChanged.Invoke();
    }

    private void OnNoteChanged(IVoiceSynthesisNote note) => MarkDirty();
    private void OnNotesChanged(IVoiceSynthesisNote note) => MarkDirty();
    private void OnRangeModified(double startTime, double endTime) => MarkDirty();

    private void SetFailed(string error)
    {
        _dirty = false;
        _failed = true;
        _error = error;
        _statusChanged.Invoke();
    }

    private bool TryGetBounds(out double startTime, out double endTime)
    {
        startTime = double.PositiveInfinity;
        endTime = double.NegativeInfinity;
        foreach (var note in _context.Notes)
        {
            startTime = Math.Min(startTime, note.StartTime.Value);
            endTime = Math.Max(endTime, note.EndTime.Value);
        }
        return double.IsFinite(startTime) && double.IsFinite(endTime);
    }

    private static List<double> CollectPitchTimes(double startTime, double endTime)
    {
        const double StepSeconds = 0.01;
        if (endTime < startTime)
        {
            throw new InvalidOperationException("Synthesis end time precedes its start time.");
        }

        var duration = endTime - startTime;
        var count = Math.Max(1, (int)Math.Ceiling(duration / StepSeconds) + 1);
        var times = new List<double>(count);
        for (var i = 0; i < count; i++)
        {
            var time = startTime + i * StepSeconds;
            times.Add(Math.Min(time, endTime));
        }
        if (times[^1] < endTime)
        {
            times.Add(endTime);
        }
        return times;
    }

    private static double[] SanitizePitchValues(IReadOnlyList<double> values)
    {
        var result = new double[values.Count];
        for (var i = 0; i < values.Count; i++)
        {
            result[i] = double.IsFinite(values[i]) ? values[i] : -double.MaxValue;
        }
        return result;
    }

    private static Dictionary<string, object?> ConvertPropertyObject(PropertyObject propertyObject)
    {
        var result = new Dictionary<string, object?>();
        foreach (var entry in propertyObject.Map)
        {
            result.Add(entry.Key, ConvertPropertyValue(entry.Value));
        }
        return result;
    }

    private static object? ConvertPropertyValue(PropertyValue value) => value.Type switch
    {
        PropertyType.Null or PropertyType.Multiple => null,
        PropertyType.Boolean => ReadBoolean(value),
        PropertyType.Number => ReadDouble(value),
        PropertyType.String => ReadString(value),
        PropertyType.Array => ReadArray(value),
        PropertyType.Object => ReadObject(value),
        _ => throw new UnreachableException(),
    };

    private static bool ReadBoolean(PropertyValue value)
    {
        value.ToBoolean(out var result);
        return result;
    }

    private static double ReadDouble(PropertyValue value)
    {
        value.ToDouble(out var result);
        return result;
    }

    private static string ReadString(PropertyValue value)
    {
        value.ToString(out var result);
        return result!;
    }

    private static object?[] ReadArray(PropertyValue value)
    {
        value.ToArray(out var array);
        return array!.Select(ConvertPropertyValue).ToArray();
    }

    private static Dictionary<string, object?> ReadObject(PropertyValue value)
    {
        value.ToObject(out var propertyObject);
        return ConvertPropertyObject(propertyObject!);
    }

    private static unsafe nint CreateNativeCancelToken() =>
      (nint)Native.NativeMethods.neutrino_tau_create_cancel_token();

    private static unsafe void CancelNativeToken(nint token) =>
      Native.NativeMethods.neutrino_tau_cancel_token_cancel((Native.CancelToken*)token);

    private static unsafe void DestroyNativeToken(nint token) =>
      Native.NativeMethods.neutrino_tau_destroy_cancel_token((Native.CancelToken*)token);

    private static double ResolveNumericPartProperty(
      PropertyObject partProperties,
      string key,
      bool roundToInteger = true)
    {
        var value = partProperties.GetDouble(key);
        if (!double.IsFinite(value))
        {
            throw new InvalidOperationException($"Part property '{key}' must be finite.");
        }
        return roundToInteger ? Math.Round(value, MidpointRounding.AwayFromZero) : value;
    }

    private static IReadOnlyList<IReadOnlyList<Point>> BuildSynthesizedPitch(
      IReadOnlyList<double> pitchTimes,
      IReadOnlyList<double> pitchValues)
    {
        if (pitchTimes.Count != pitchValues.Count)
        {
            throw new InvalidOperationException("Native synthesis returned mismatched pitch arrays.");
        }
        if (pitchTimes.Count == 0)
        {
            return [];
        }

        var line = new List<Point>(pitchTimes.Count);
        for (var i = 0; i < pitchTimes.Count; i++)
        {
            if (double.IsFinite(pitchTimes[i]) && double.IsFinite(pitchValues[i]))
            {
                line.Add(new Point(pitchTimes[i], pitchValues[i]));
            }
        }
        return line.Count == 0 ? [] : [line];
    }

    private static IReadOnlyMap<string, SynthesizedSyllable> BuildSynthesizedPhonemes(
      IReadOnlyList<VoiceSynthesisNoteSnapshot> notes,
      IReadOnlyList<NotePhonemesPayload> notePhonemes)
    {
        var result = new Map<string, SynthesizedSyllable>();
        foreach (var entry in notePhonemes)
        {
            if (entry.NoteIndex < 0 || entry.NoteIndex >= notes.Count)
            {
                throw new InvalidOperationException("Native synthesis returned an invalid note index.");
            }

            var note = notes[entry.NoteIndex];
            if (entry.Phonemes.Length == 0)
            {
                result.Add(note.Id, new SynthesizedSyllable([], [], 0));
                continue;
            }

            var leadingCount = 0;
            foreach (var phoneme in entry.Phonemes)
            {
                if ((phoneme.StartTime + phoneme.EndTime) / 2 < note.StartTime)
                {
                    leadingCount++;
                }
                else
                {
                    break;
                }
            }
            if (leadingCount == entry.Phonemes.Length)
            {
                leadingCount--;
            }

            var leading = new List<SynthesizedPhoneme>(leadingCount);
            var body = new List<SynthesizedPhoneme>(entry.Phonemes.Length - leadingCount);
            for (var i = 0; i < entry.Phonemes.Length; i++)
            {
                var phoneme = entry.Phonemes[i];
                var synthesized = new SynthesizedPhoneme
                {
                    Symbol = phoneme.Symbol,
                    Duration = Math.Max(0, phoneme.EndTime - phoneme.StartTime),
                    StretchWeight = i == leadingCount ? 1 : 0,
                };
                if (i < leadingCount)
                {
                    leading.Add(synthesized);
                }
                else
                {
                    body.Add(synthesized);
                }
            }

            result.Add(
              note.Id,
              new SynthesizedSyllable(
                leading,
                body,
                entry.Phonemes[leadingCount].StartTime - note.StartTime
              )
            );
        }
        return result;
    }

    private sealed class SynthesisTaskPayload
    {
        public required string VoiceId { get; init; }
        public double StartTime { get; init; }
        public double EndTime { get; init; }
        public double Duration { get; init; }
        public double StyleShift { get; init; }
        public double WaveformStyleShift { get; init; }
        public double PitchShiftCents { get; init; }
        public required Dictionary<string, object?> PartProperties { get; init; }
        public required List<SynthesisNotePayload> Notes { get; init; }
        public required PitchPayload Pitch { get; init; }
    }

    private sealed class SynthesisNotePayload
    {
        public double StartTime { get; init; }
        public double EndTime { get; init; }
        public int Pitch { get; init; }
        public required string Lyric { get; init; }
        public int? LastIndex { get; init; }
        public int? NextIndex { get; init; }
        public required Dictionary<string, object?> Properties { get; init; }
        public required List<SynthesisPhonemePayload> Phonemes { get; init; }
    }

    private sealed class SynthesisPhonemePayload
    {
        public required string Symbol { get; init; }
        public double StartTime { get; init; }
        public double EndTime { get; init; }
    }

    private sealed class PitchPayload
    {
        public required List<double> Times { get; init; }
        public required double[] Values { get; init; }
    }

    private sealed class SynthesisResponse
    {
        public double StartTime { get; init; }
        public int SampleRate { get; init; }
        public int SampleCount { get; init; }
        public float[] Samples { get; init; } = [];
        public double[] PitchTimes { get; init; } = [];
        public double[] PitchValues { get; init; } = [];
        public NotePhonemesPayload[] NotePhonemes { get; init; } = [];
    }

    private sealed class NotePhonemesPayload
    {
        public int NoteIndex { get; init; }
        public SynthesisPhonemePayload[] Phonemes { get; init; } = [];
    }

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };
    private static readonly SynthesizedPitch EmptySynthesizedPitch = new() { Segments = [] };
    private static readonly IReadOnlyMap<string, SynthesizedParameter> EmptySynthesizedParameters =
      Map<string, SynthesizedParameter>.Empty;

    private readonly IVoiceSynthesisContext _context;
    private readonly string _voiceId;
    private readonly nint _nativeEngine;
    private readonly DisposableManager _subscriptions = new();
    private readonly IActionEvent<IVoiceSynthesisNote> _noteChanged;
    private readonly object _cancelTokenLock = new();
    private readonly ActionEvent _synthesizedPhonemesChanged = new();
    private readonly ActionEvent _synthesizedPitchChanged = new();
    private readonly ActionEvent _statusChanged = new();

    private SynthesizedPitch _synthesizedPitch = EmptySynthesizedPitch;
    private IReadOnlyMap<string, SynthesizedSyllable> _synthesizedPhonemes =
      Map<string, SynthesizedSyllable>.Empty;
    private IAudioSegment? _audioSegment;
    private nint _activeCancelToken;
    private bool _dirty;
    private bool _failed;
    private bool _synthesizing;
    private bool _hasResult;
    private bool _disposed;
    private string? _error;
    private double _progress;
}
