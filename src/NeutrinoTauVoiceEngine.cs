using System.Runtime.InteropServices;
using System.Collections.Generic;
using TuneLab.Base.Properties;
using TuneLab.Base.Structures;
using TuneLab.Extensions.Voices;
using TuneLab.Base.Utils;

namespace NeutrinoTau;

[VoiceEngine("neutrino-tau")]
public unsafe class NeutrinoTauVoiceEngine : IVoiceEngine
{
    public IReadOnlyOrderedMap<string, VoiceSourceInfo> VoiceInfos =>
        new OrderedMap<string, VoiceSourceInfo>();

    private Native.CEngine* _nativeEngine;

    public IVoiceSource CreateVoiceSource(string id)
    {
        return new NeutrinoTauVoiceSource(id);
    }

    public void Destroy()
    {
        if (_nativeEngine != null)
        {
            Native.NativeMethods.neutrino_tau_destroy_engine(_nativeEngine);
            _nativeEngine = null;
        }
    }

    public unsafe bool Init(string enginePath, out string? error)
    {
        Log.Info($"Initializing Neutrino Tau Voice Engine with path: {enginePath}");

        var dllPathBytes = System.Text.Encoding.UTF8.GetBytes(enginePath + "\0");
        fixed (byte* dllPathPtr = dllPathBytes)
        {
            byte* errorPtr = null;
            _nativeEngine = Native.NativeMethods.neutrino_tau_create_engine(dllPathPtr, &errorPtr);
            if (_nativeEngine == null)
            {
                error = errorPtr != null ? Marshal.PtrToStringUTF8((IntPtr)errorPtr) : "Unknown error";
                if (errorPtr != null)
                {
                    Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
                }
                return false;
            }
        }

        error = null;
        return true;
    }

    private sealed class NeutrinoTauVoiceSource : IVoiceSource
    {
        public string Name => string.IsNullOrEmpty(_id) ? VoiceSource.Name : _id;
        public string DefaultLyric { get; } = "a";
        public IReadOnlyOrderedMap<string, AutomationConfig> AutomationConfigs => AutomationConfigMap;
        public IReadOnlyOrderedMap<string, IPropertyConfig> PartProperties => PartPropertyMap;
        public IReadOnlyOrderedMap<string, IPropertyConfig> NoteProperties => NotePropertyMap;

        public NeutrinoTauVoiceSource(string id)
        {
            _id = id;
        }

        public IReadOnlyList<SynthesisSegment<T>> Segment<T>(SynthesisSegment<T> segment) where T : ISynthesisNote
        {
            return this.SimpleSegment(segment);
        }

        public ISynthesisTask CreateSynthesisTask(ISynthesisData data)
        {
            return new NeutrinoTauSynthesisTask(data);
        }

        private readonly string _id;
    }

    private static readonly OrderedMap<string, AutomationConfig> AutomationConfigMap = new();
    private static readonly OrderedMap<string, IPropertyConfig> PartPropertyMap = new();
    private static readonly OrderedMap<string, IPropertyConfig> NotePropertyMap = new();
    private static readonly VoiceSourceInfo VoiceSource = new()
    {
        Name = "Neutrino Tau",
        Description = "Scaffold voice source for extension development."
    };
}
