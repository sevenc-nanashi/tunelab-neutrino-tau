using System.Runtime.InteropServices;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;
using TuneLab.Base.Properties;
using TuneLab.Base.Structures;
using TuneLab.Extensions.Voices;
using TuneLab.Base.Utils;

namespace NeutrinoTau;

[VoiceEngine("neutrino-tau")]
public unsafe class NeutrinoTauVoiceEngine : IVoiceEngine
{
    public IReadOnlyOrderedMap<string, VoiceSourceInfo> VoiceInfos => _voiceInfos;

    private Native.CEngine* _nativeEngine;
    private readonly OrderedMap<string, VoiceSourceInfo> _voiceInfos = new();

    public IVoiceSource CreateVoiceSource(string id)
    {
        return new NeutrinoTauVoiceSource(id, this);
    }

    public void Destroy()
    {
        if (_nativeEngine != null)
        {
            Native.NativeMethods.neutrino_tau_destroy_engine(_nativeEngine);
            _nativeEngine = null;
        }
        _voiceInfos.Clear();
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

        if (!LoadVoiceSources(out error))
        {
            Destroy();
            return false;
        }

        error = null;
        return true;
    }

    private bool LoadVoiceSources(out string? error)
    {
        if (_nativeEngine == null)
        {
            error = "Engine is not initialized.";
            return false;
        }

        byte* errorPtr = null;
        var voicesJsonPtr = Native.NativeMethods.neutrino_tau_load_voice_sources_json(_nativeEngine, &errorPtr);
        if (voicesJsonPtr == null)
        {
            error = errorPtr != null ? Marshal.PtrToStringUTF8((IntPtr)errorPtr) : "Failed to load voice sources.";
            if (errorPtr != null)
            {
                Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
            }
            return false;
        }

        try
        {
            var voicesJson = Marshal.PtrToStringUTF8((IntPtr)voicesJsonPtr);
            if (voicesJson == null)
            {
                error = "Failed to decode voice source payload.";
                return false;
            }

            var voices = JsonSerializer.Deserialize<List<NativeVoiceSource>>(voicesJson) ?? [];
            _voiceInfos.Clear();
            foreach (var voice in voices)
            {
                if (string.IsNullOrWhiteSpace(voice.Id) || string.IsNullOrWhiteSpace(voice.Name))
                {
                    continue;
                }

                _voiceInfos.Add(
                    voice.Id,
                    new VoiceSourceInfo
                    {
                        Name = voice.Name,
                        Description = voice.Description ?? string.Empty,
                    }
                );
            }

            error = null;
            return true;
        }
        catch (Exception ex)
        {
            error = $"Failed to parse voice source payload: {ex.Message}";
            return false;
        }
        finally
        {
            Native.NativeMethods.neutrino_tau_free_c_string(voicesJsonPtr);
            if (errorPtr != null)
            {
                Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
            }
        }
    }

    private sealed class NeutrinoTauVoiceSource : IVoiceSource
    {
        public string Name => string.IsNullOrEmpty(_id) ? DefaultVoiceSource.Name : _id;
        public string DefaultLyric { get; } = "a";
        public IReadOnlyOrderedMap<string, AutomationConfig> AutomationConfigs => AutomationConfigMap;
        public IReadOnlyOrderedMap<string, IPropertyConfig> PartProperties => PartPropertyMap;
        public IReadOnlyOrderedMap<string, IPropertyConfig> NoteProperties => NotePropertyMap;

        public NeutrinoTauVoiceSource(string id, NeutrinoTauVoiceEngine owner)
        {
            _id = id;
            _owner = owner;
        }

        public IReadOnlyList<SynthesisSegment<T>> Segment<T>(SynthesisSegment<T> segment) where T : ISynthesisNote
        {
            return this.SimpleSegment(segment);
        }

        public ISynthesisTask CreateSynthesisTask(ISynthesisData data)
        {
            return new NeutrinoTauSynthesisTask(data, _owner._nativeEngine, _id);
        }

        private readonly string _id;
        private readonly NeutrinoTauVoiceEngine _owner;
    }

    private static readonly OrderedMap<string, AutomationConfig> AutomationConfigMap = new();
    private static readonly OrderedMap<string, IPropertyConfig> PartPropertyMap = new()
    {
        { "styleShift", new NumberConfig(0.0, -24.0, 24.0, true) },
        { "waveformStyleShift", new NumberConfig(0.0, -24.0, 24.0, true) },
    };
    private static readonly OrderedMap<string, IPropertyConfig> NotePropertyMap = new();
    private sealed class NativeVoiceSource
    {
        [JsonPropertyName("id")]
        public string? Id { get; init; }

        [JsonPropertyName("name")]
        public string? Name { get; init; }

        [JsonPropertyName("description")]
        public string? Description { get; init; }
    }

    private static readonly VoiceSourceInfo DefaultVoiceSource = new()
    {
        Name = "Neutrino Tau",
        Description = "Neutrino Tau voice source for extension development."
    };
}
