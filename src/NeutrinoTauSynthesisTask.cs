using System;
using NeutrinoTau.Native;
using TuneLab.Extensions.Voices;

namespace NeutrinoTau;

public sealed class NeutrinoTauSynthesisTask : ISynthesisTask
{
  public event Action<SynthesisResult>? Complete;
  public event Action<double>? Progress;
  public event Action<string>? Error;

  public NeutrinoTauSynthesisTask(ISynthesisData data)
  {
    _startTime = data.StartTime();
    _endTime = data.EndTime();
  }

  public void Start()
  {
    try
    {
      var sampleRate = NativeMethods.neutrino_tau_default_sample_rate();
      var duration = Math.Max(0.0, _endTime - _startTime);
      var sampleCount = NativeMethods.neutrino_tau_calculate_sample_count(duration, sampleRate);

      Progress?.Invoke(1.0);
      Complete?.Invoke(new SynthesisResult(_startTime, sampleRate, new float[sampleCount]));
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

  private readonly double _startTime;
  private readonly double _endTime;
}
