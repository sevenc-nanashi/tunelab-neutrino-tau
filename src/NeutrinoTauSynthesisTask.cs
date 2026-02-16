using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using NeutrinoTau.Native;
using TuneLab.Base.Properties;
using TuneLab.Extensions.Voices;

namespace NeutrinoTau;

public sealed class NeutrinoTauSynthesisTask : ISynthesisTask
{
  public event Action<SynthesisResult>? Complete;
  public event Action<double>? Progress;
  public event Action<string>? Error;

  public NeutrinoTauSynthesisTask(ISynthesisData data)
  {
    _data = data;
    _notes = data.Notes.ToList();
    if (_notes.Count == 0)
    {
      _startTime = 0.0;
      _endTime = 0.0;
      return;
    }

    _startTime = _notes[0].StartTime;
    _endTime = _notes[^1].EndTime;
  }

  public unsafe void Start()
  {
    try
    {
      var payload = BuildPayload();
      var payloadJson = JsonSerializer.Serialize(payload, JsonOptions);
      var payloadBytes = Encoding.UTF8.GetBytes(payloadJson + "\0");
      byte* errorPtr = null;
      byte* resultPtr = null;

      fixed (byte* payloadPtr = payloadBytes)
      {
        resultPtr = NativeMethods.neutrino_tau_scaffold_synthesis_task(payloadPtr, &errorPtr);
      }

      if (resultPtr == null)
      {
        var err = errorPtr != null ? Marshal.PtrToStringUTF8((IntPtr)errorPtr) : "Unknown native error";
        if (errorPtr != null)
        {
          NativeMethods.neutrino_tau_free_c_string(errorPtr);
        }
        throw new InvalidOperationException(err ?? "Unknown native error");
      }

      ScaffoldSynthesisResponse? response;
      try
      {
        var resultJson = Marshal.PtrToStringUTF8((IntPtr)resultPtr);
        if (string.IsNullOrWhiteSpace(resultJson))
        {
          throw new InvalidOperationException("Native scaffold response is empty.");
        }

        response = JsonSerializer.Deserialize<ScaffoldSynthesisResponse>(resultJson, JsonOptions);
      }
      finally
      {
        NativeMethods.neutrino_tau_free_c_string(resultPtr);
        if (errorPtr != null)
        {
          NativeMethods.neutrino_tau_free_c_string(errorPtr);
        }
      }

      if (response == null)
      {
        throw new InvalidOperationException("Failed to parse native scaffold response.");
      }

      Progress?.Invoke(1.0);
      Complete?.Invoke(new SynthesisResult(_startTime, response.SampleRate, new float[response.SampleCount]));
    }
    catch (Exception ex)
    {
      Error?.Invoke($"Native synthesis scaffold failed: {ex.Message}");
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
  }

  public void SetDirty(string dirtyType)
  {
    // Ignore in scaffold implementation.
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
      Phonemes = note.Phonemes.Select(phoneme => new SynthesisPhonemePayload
      {
        Symbol = phoneme.Symbol,
        StartTime = phoneme.StartTime,
        EndTime = phoneme.EndTime,
      }).ToList(),
    }).ToList();

    var pitchTimes = CollectPitchTimes(notePayloads);
    var pitchValues = _data.Pitch.GetValue(pitchTimes);

    return new SynthesisTaskPayload
    {
      StartTime = _startTime,
      EndTime = _endTime,
      Duration = Math.Max(0.0, _endTime - _startTime),
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

  private static List<double> CollectPitchTimes(IReadOnlyList<SynthesisNotePayload> notes)
  {
    var times = new SortedSet<double>();
    foreach (var note in notes)
    {
      times.Add(note.StartTime);
      times.Add(note.EndTime);
      foreach (var phoneme in note.Phonemes)
      {
        times.Add(phoneme.StartTime);
        times.Add(phoneme.EndTime);
      }
    }

    if (times.Count == 0)
    {
      times.Add(0.0);
    }

    return times.ToList();
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

  private sealed class SynthesisTaskPayload
  {
    public double StartTime { get; init; }
    public double EndTime { get; init; }
    public double Duration { get; init; }
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

  private sealed class ScaffoldSynthesisResponse
  {
    public int SampleRate { get; init; }
    public int SampleCount { get; init; }
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
  private readonly List<ISynthesisNote> _notes;
  private readonly double _startTime;
  private readonly double _endTime;
}
