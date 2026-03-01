using System.Globalization;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using TuneLab.Base.Properties;
using TuneLab.Base.Structures;
using TuneLab.Extensions.Voices;

namespace NeutrinoTau;

public unsafe sealed class NeutrinoTauSynthesisTask : ISynthesisTask
{
  public event Action<SynthesisResult>? Complete;
  public event Action<double>? Progress;
  public event Action<string>? Error;

  public NeutrinoTauSynthesisTask(ISynthesisData data)
    : this(data, null, string.Empty)
  {
  }

  internal NeutrinoTauSynthesisTask(ISynthesisData data, Native.CEngine* nativeEngine, string voiceId)
  {
    _data = data;
    _nativeEngine = nativeEngine;
    _voiceId = voiceId ?? string.Empty;
    _notes = [.. data.Notes];
    if (_notes.Count == 0)
    {
      _startTime = 0.0;
      _endTime = 0.0;
      return;
    }

    _startTime = _notes[0].StartTime;
    _endTime = _notes[^1].EndTime;
  }

  public void Start()
  {
    lock (_taskLock)
    {
      if (_runningTask is { IsCompleted: false })
      {
        return;
      }

      _cancellationTokenSource?.Cancel();
      _cancellationTokenSource?.Dispose();
      _cancellationTokenSource = new CancellationTokenSource();
      if (_nativeCancelToken != null)
      {
        Native.NativeMethods.neutrino_tau_destroy_cancel_token(_nativeCancelToken);
        _nativeCancelToken = null;
      }

      _nativeCancelToken = Native.NativeMethods.neutrino_tau_create_cancel_token();
      if (_nativeCancelToken == null)
      {
        _cancellationTokenSource.Dispose();
        _cancellationTokenSource = null;
        Error?.Invoke("Failed to create native cancel token.");
        return;
      }

      var token = _cancellationTokenSource.Token;
      var nativeCancelToken = _nativeCancelToken;
      _runningTask = Task.Run(() => RunSynthesis(token, nativeCancelToken), token);
    }
  }

  public void Suspend()
  {
  }

  public void Resume()
  {
  }

  public void Stop()
  {
    lock (_taskLock)
    {
      _cancellationTokenSource?.Cancel();
      if (_nativeCancelToken != null)
      {
        Native.NativeMethods.neutrino_tau_cancel_token_cancel(_nativeCancelToken);
      }
    }
  }

  public void SetDirty(string dirtyType)
  {
    Stop();
  }

  private SynthesisTaskPayload BuildPayload()
  {
    var noteIndexMap = new Dictionary<ISynthesisNote, int>(SynthesisNoteReferenceComparer.Instance);
    for (var i = 0; i < _notes.Count; i++)
    {
      noteIndexMap[_notes[i]] = i;
    }

    var notePayloads = _notes.Select(note => new SynthesisNotePayload
    {
      StartTime = note.StartTime,
      EndTime = note.EndTime,
      Pitch = note.Pitch,
      Lyric = note.Lyric,
      LastIndex = ResolveNeighborIndex(note.Last, noteIndexMap),
      NextIndex = ResolveNeighborIndex(note.Next, noteIndexMap),
      Properties = ConvertPropertyObject(note.Properties),
      Phonemes = [.. note.Phonemes.Select(phoneme => new SynthesisPhonemePayload
      {
        Symbol = phoneme.Symbol,
        StartTime = phoneme.StartTime,
        EndTime = phoneme.EndTime,
      })],
    }).ToList();

    var pitchTimes = CollectPitchTimes(_startTime, _endTime);
    var pitchValues = SanitizePitchValues(_data.Pitch.GetValue(pitchTimes));

    return new SynthesisTaskPayload
    {
      VoiceId = _voiceId,
      StartTime = _startTime,
      EndTime = _endTime,
      Duration = Math.Max(0.0, _endTime - _startTime),
      StyleShift = ResolveNumericPartProperty(_data.PartProperties, "styleshift"),
      WaveformStyleShift = ResolveNumericPartProperty(_data.PartProperties, "waveformstyleshift"),
      PartProperties = ConvertPropertyObject(_data.PartProperties),
      Notes = notePayloads,
      Pitch = new PitchPayload
      {
        Times = pitchTimes,
        Values = pitchValues,
      },
    };
  }

  private static int? ResolveNeighborIndex(ISynthesisNote? note, IReadOnlyDictionary<ISynthesisNote, int> noteIndexMap)
  {
    if (note == null)
    {
      return null;
    }

    return noteIndexMap.TryGetValue(note, out var index) ? index : null;
  }

  private static List<double> CollectPitchTimes(double startTime, double endTime)
  {
    const double stepSeconds = 0.01; // 10ms
    if (!double.IsFinite(startTime) || !double.IsFinite(endTime))
    {
      return [0.0];
    }

    if (endTime < startTime)
    {
      (startTime, endTime) = (endTime, startTime);
    }

    var duration = Math.Max(0.0, endTime - startTime);
    var count = Math.Max(1, (int)Math.Ceiling(duration / stepSeconds) + 1);
    var times = new List<double>(count);
    for (var i = 0; i < count; i++)
    {
      var t = startTime + i * stepSeconds;
      times.Add(t > endTime ? endTime : t);
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
    foreach (var kv in propertyObject.Map)
    {
      result[kv.Key] = ConvertPropertyValue(kv.Value);
    }
    return result;
  }

  private static object? ConvertPropertyValue(PropertyValue value)
  {
    if (value.IsInvalid())
    {
      return null;
    }

    if (value.ToBool(out var boolValue))
    {
      return boolValue;
    }

    if (value.ToDouble(out var numberValue))
    {
      return numberValue;
    }

    if (value.ToString(out var stringValue))
    {
      return stringValue;
    }

    if (value.ToObject(out var objectValue))
    {
      return ConvertPropertyObject(objectValue);
    }

    return value.ToString();
  }

  private static double ResolveNumericPartProperty(PropertyObject partProperties, string normalizedTargetKey)
  {
    foreach (var kv in partProperties.Map)
    {
      var key = kv.Key?.ToString();
      if (string.IsNullOrWhiteSpace(key))
      {
        continue;
      }

      var normalized = key
        .Replace("_", string.Empty, StringComparison.Ordinal)
        .Replace(" ", string.Empty, StringComparison.Ordinal)
        .Replace("-", string.Empty, StringComparison.Ordinal)
        .ToLowerInvariant();
      if (normalized != normalizedTargetKey)
      {
        continue;
      }

      var value = kv.Value;
      if (value.ToDouble(out var d) && double.IsFinite(d))
      {
        return Math.Round(d, MidpointRounding.AwayFromZero);
      }

      if (value.ToString(out var s)
          && double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out var parsed)
          && double.IsFinite(parsed))
      {
        return Math.Round(parsed, MidpointRounding.AwayFromZero);
      }
    }

    return 0.0;
  }

  private sealed class SynthesisTaskPayload
  {
    public string VoiceId { get; init; } = string.Empty;
    public double StartTime { get; init; }
    public double EndTime { get; init; }
    public double Duration { get; init; }
    public double StyleShift { get; init; }
    public double WaveformStyleShift { get; init; }
    public Dictionary<string, object?> PartProperties { get; init; } = [];
    public List<SynthesisNotePayload> Notes { get; init; } = [];
    public PitchPayload Pitch { get; init; } = new();
  }

  private sealed class SynthesisNotePayload
  {
    public double StartTime { get; init; }
    public double EndTime { get; init; }
    public int Pitch { get; init; }
    public string Lyric { get; init; } = string.Empty;
    public int? LastIndex { get; init; }
    public int? NextIndex { get; init; }
    public Dictionary<string, object?> Properties { get; init; } = [];
    public List<SynthesisPhonemePayload> Phonemes { get; init; } = [];
  }

  private sealed class SynthesisPhonemePayload
  {
    public string Symbol { get; init; } = string.Empty;
    public double StartTime { get; init; }
    public double EndTime { get; init; }
  }

  private sealed class PitchPayload
  {
    public List<double> Times { get; init; } = [];
    public double[] Values { get; init; } = [];
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

  private sealed class SynthesisNoteReferenceComparer : IEqualityComparer<ISynthesisNote>
  {
    public static SynthesisNoteReferenceComparer Instance { get; } = new();

    public bool Equals(ISynthesisNote? x, ISynthesisNote? y)
    {
      return ReferenceEquals(x, y);
    }

    public int GetHashCode(ISynthesisNote obj)
    {
      return obj == null ? 0 : RuntimeHelpers.GetHashCode(obj);
    }
  }

  private readonly ISynthesisData _data;
  private readonly Native.CEngine* _nativeEngine;
  private readonly string _voiceId;
  private readonly List<ISynthesisNote> _notes;
  private readonly double _startTime;
  private readonly double _endTime;
  private readonly object _taskLock = new();
  private CancellationTokenSource? _cancellationTokenSource;
  private Task? _runningTask;
  private Native.CancelToken* _nativeCancelToken;

  private void RunSynthesis(CancellationToken token, Native.CancelToken* nativeCancelToken)
  {
    try
    {
      var payload = BuildPayload();
      if (_nativeEngine == null)
      {
        throw new InvalidOperationException("Native engine is not initialized.");
      }
      token.ThrowIfCancellationRequested();
      var payloadJson = JsonSerializer.Serialize(payload, JsonOptions);
      var payloadBytes = Encoding.UTF8.GetBytes(payloadJson + "\0");
      byte* errorPtr = null;
      byte* resultPtr = null;

      try
      {
        fixed (byte* payloadPtr = payloadBytes)
        {
          resultPtr = Native.NativeMethods.neutrino_tau_synthesize(_nativeEngine, payloadPtr, nativeCancelToken, &errorPtr);
        }

        token.ThrowIfCancellationRequested();

        if (resultPtr == null)
        {
          var err = errorPtr != null ? Marshal.PtrToStringUTF8((IntPtr)errorPtr) : "Unknown native error";
          throw new InvalidOperationException(err ?? "Unknown native error");
        }

        var resultJson = Marshal.PtrToStringUTF8((IntPtr)resultPtr);
        if (string.IsNullOrWhiteSpace(resultJson))
        {
          throw new InvalidOperationException("Native synthesis response is empty.");
        }

        var response = JsonSerializer.Deserialize<SynthesisResponse>(resultJson, JsonOptions) ?? throw new InvalidOperationException("Failed to parse native synthesis response.");
        token.ThrowIfCancellationRequested();

        var samples = response.Samples.Length > 0 ? response.Samples : new float[Math.Max(0, response.SampleCount)];
        var synthesizedPitch = BuildSynthesizedPitch(response.PitchTimes, response.PitchValues);
        var synthesizedPhonemes = BuildSynthesizedPhonemes(_notes, response.NotePhonemes);
        Progress?.Invoke(1.0);
        Complete?.Invoke(new SynthesisResult(response.StartTime, response.SampleRate, samples, synthesizedPitch, synthesizedPhonemes));
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
    catch (OperationCanceledException)
    {
      // Task was canceled by Stop/SetDirty.
    }
    catch (Exception ex)
    {
      if (!token.IsCancellationRequested)
      {
        Error?.Invoke($"Native synthesis failed: {ex.Message}");
      }
    }
    finally
    {
      lock (_taskLock)
      {
        if (_nativeCancelToken == nativeCancelToken)
        {
          Native.NativeMethods.neutrino_tau_destroy_cancel_token(_nativeCancelToken);
          _nativeCancelToken = null;
        }
      }
    }
  }

  private static IReadOnlyList<IReadOnlyList<Point>> BuildSynthesizedPitch(
    IReadOnlyList<double> pitchTimes,
    IReadOnlyList<double> pitchValues)
  {
    var count = Math.Min(pitchTimes.Count, pitchValues.Count);
    if (count == 0)
    {
      return [];
    }

    var line = new List<Point>(count);
    for (var i = 0; i < count; i++)
    {
      var x = pitchTimes[i];
      var y = pitchValues[i];
      if (!double.IsFinite(x) || !double.IsFinite(y))
      {
        continue;
      }
      line.Add(new Point(x, y));
    }

    return line.Count == 0 ? [] : [line];
  }

  private static IReadOnlyDictionary<ISynthesisNote, SynthesizedPhoneme[]> BuildSynthesizedPhonemes(
    IReadOnlyList<ISynthesisNote> notes,
    IReadOnlyList<NotePhonemesPayload> notePhonemes)
  {
    if (notes.Count == 0)
    {
      return new Dictionary<ISynthesisNote, SynthesizedPhoneme[]>(SynthesisNoteReferenceComparer.Instance);
    }

    var map = new Dictionary<ISynthesisNote, SynthesizedPhoneme[]>(SynthesisNoteReferenceComparer.Instance);

    foreach (var entry in notePhonemes)
    {
      if (entry.NoteIndex < 0 || entry.NoteIndex >= notes.Count)
      {
        continue;
      }
      var mapped = entry.Phonemes
        .Where(p =>
          !string.IsNullOrWhiteSpace(p.Symbol)
          && double.IsFinite(p.StartTime)
          && double.IsFinite(p.EndTime)
          && p.EndTime > p.StartTime)
        .Select(p => new SynthesizedPhoneme
        {
          Symbol = p.Symbol,
          StartTime = p.StartTime,
          EndTime = p.EndTime,
        })
        .ToArray();
      if (mapped.Length > 0)
      {
        map[notes[entry.NoteIndex]] = mapped;
      }
    }
    return map;
  }
}
