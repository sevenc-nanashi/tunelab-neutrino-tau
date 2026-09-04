using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;
using TuneLab.Foundation;
using TuneLab.SDK;

namespace NeutrinoTau;

public unsafe sealed class NeutrinoTauVoiceEngine : IVoiceSynthesisEngine, IExtensionSettings
{
    public IReadOnlyOrderedMap<string, VoiceSourceInfo> VoiceSourceInfos => _voiceSourceInfos;

    public ObjectConfig GetSettingsConfig(IExtensionSettingsContext context)
    {
        var properties = new OrderedMap<PropertyKey, IControllerConfig>
        {
            { (NeutrinoPathSettingKey, "NEUTRINO Path"), TextBoxConfig.Create(string.Empty) },
        };
        return ObjectConfig.Create(properties);
    }

    public void ApplySettings(PropertyObject settings)
    {
        var neutrinoPath = settings.GetString(NeutrinoPathSettingKey, string.Empty);
        if (_nativeEngine != null && neutrinoPath != _neutrinoPath)
        {
            TuneLabContext.Global.GetLogger().Info(
              "The updated NEUTRINO path will be applied after restarting TuneLab."
            );
        }
        _neutrinoPath = neutrinoPath;
    }

    public void Init()
    {
        if (string.IsNullOrWhiteSpace(_neutrinoPath))
        {
            throw new InvalidOperationException(
              "NEUTRINO path is not configured. Set it in Settings > Extensions."
            );
        }

        var enginePath = Path.GetDirectoryName(typeof(NeutrinoTauVoiceEngine).Assembly.Location);
        if (enginePath == null)
        {
            throw new InvalidOperationException("Failed to locate the Neutrino Tau extension directory.");
        }

        TuneLabContext.Global.GetLogger().Info($"Initializing Neutrino Tau voice engine at: {enginePath}");

        var enginePathBytes = System.Text.Encoding.UTF8.GetBytes(enginePath + "\0");
        var neutrinoPathBytes = System.Text.Encoding.UTF8.GetBytes(_neutrinoPath + "\0");
        fixed (byte* enginePathPtr = enginePathBytes)
        fixed (byte* neutrinoPathPtr = neutrinoPathBytes)
        {
            byte* errorPtr = null;
            _nativeEngine = Native.NativeMethods.neutrino_tau_create_engine(
              enginePathPtr,
              neutrinoPathPtr,
              &errorPtr
            );
            if (_nativeEngine == null)
            {
                var error = errorPtr != null ? Marshal.PtrToStringUTF8((IntPtr)errorPtr) : "Unknown error";
                if (errorPtr != null)
                {
                    Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
                }
                throw new InvalidOperationException(error);
            }
        }

        try
        {
            LoadVoiceSources();
        }
        catch
        {
            Destroy();
            throw;
        }
    }

    public void Destroy()
    {
        if (_nativeEngine != null)
        {
            Native.NativeMethods.neutrino_tau_destroy_engine(_nativeEngine);
            _nativeEngine = null;
        }
        _voiceSourceInfos.Clear();
    }

    public IVoiceSynthesisSession CreateSession(IVoiceSynthesisContext context)
    {
        if (_nativeEngine == null)
        {
            throw new InvalidOperationException("Native engine is not initialized.");
        }
        return new NeutrinoTauSynthesisSession(context, _nativeEngine);
    }

    public IReadOnlyOrderedMap<PropertyKey, AutomationConfig> GetAutomationConfigs(
      IVoiceSynthesisPartPropertyContext context) => EmptyAutomationConfigs;

    public IReadOnlyOrderedMap<PropertyKey, AutomationConfig> GetSynthesizedParameterConfigs(
      IVoiceSynthesisPartPropertyContext context) => EmptyAutomationConfigs;

    public ObjectConfig GetPartPropertyConfig(IVoiceSynthesisPartPropertyContext context) =>
      PartPropertyConfig;

    public ObjectConfig GetNotePropertyConfig(IVoiceSynthesisNotePropertyContext context) =>
      EmptyPropertyConfig;

    public IReadOnlyMap<int, ObjectConfig> GetPhonemePropertyConfigs(
      IVoiceSynthesisNotePropertyContext context) => EmptyPhonemePropertyConfigs;

    private void LoadVoiceSources()
    {
        if (_nativeEngine == null)
        {
            throw new InvalidOperationException("Native engine is not initialized.");
        }

        byte* errorPtr = null;
        var voicesJsonPtr = Native.NativeMethods.neutrino_tau_load_voice_sources_json(_nativeEngine, &errorPtr);
        if (voicesJsonPtr == null)
        {
            var error = errorPtr != null
              ? Marshal.PtrToStringUTF8((IntPtr)errorPtr)
              : "Failed to load voice sources.";
            if (errorPtr != null)
            {
                Native.NativeMethods.neutrino_tau_free_c_string(errorPtr);
            }
            throw new InvalidOperationException(error);
        }

        try
        {
            var voicesJson = Marshal.PtrToStringUTF8((IntPtr)voicesJsonPtr);
            if (voicesJson == null)
            {
                throw new InvalidOperationException("Failed to decode the voice source payload.");
            }

            var voices = JsonSerializer.Deserialize<List<NativeVoiceSource>>(voicesJson);
            if (voices == null)
            {
                throw new JsonException("Failed to parse the voice source payload.");
            }

            _voiceSourceInfos.Clear();
            foreach (var voice in voices)
            {
                _voiceSourceInfos.Add(
                  voice.Id,
                  new VoiceSourceInfo
                  {
                      Name = voice.Name,
                      Description = voice.Description,
                  }
                );
            }
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

    private sealed class NativeVoiceSource
    {
        [JsonPropertyName("id")]
        public required string Id { get; init; }

        [JsonPropertyName("name")]
        public required string Name { get; init; }

        [JsonPropertyName("description")]
        public required string Description { get; init; }
    }

    private Native.CEngine* _nativeEngine;
    private string _neutrinoPath = string.Empty;
    private readonly OrderedMap<string, VoiceSourceInfo> _voiceSourceInfos = [];

    private const string NeutrinoPathSettingKey = "neutrino_path";
    private static readonly OrderedMap<PropertyKey, AutomationConfig> EmptyAutomationConfigs = [];
    private static readonly ObjectConfig PartPropertyConfig = ObjectConfig.Create(
      new OrderedMap<PropertyKey, IControllerConfig>
      {
      { "styleShift", SliderConfig.Integer(0, -24, 24) },
      { "waveformStyleShift", SliderConfig.Integer(0, -24, 24) },
      { "pitchShiftCents", SliderConfig.Linear(0, -2400, 2400) },
      }
    );
    private static readonly ObjectConfig EmptyPropertyConfig = ObjectConfig.Create([]);
    private static readonly IReadOnlyMap<int, ObjectConfig> EmptyPhonemePropertyConfigs =
      Map<int, ObjectConfig>.Empty;
}
