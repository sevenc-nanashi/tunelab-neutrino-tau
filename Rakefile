# frozen_string_literal: true


task :link do
  target_destination = ENV["APPDATA"] + "/TuneLab/Extensions/tunelab-neutrino-tau"
  output_dir = "#{__dir__}/bin/Debug/net8.0"
  nuget_dir = ENV["USERPROFILE"] + "/.nuget/packages"
  mkdir_p target_destination
  ln_s "#{__dir__}/description.json", target_destination + "/description.json", force: true
  Dir.glob("./bin/Debug/net8.0/*.dll").each do |dll|
    ln_s File.expand_path(dll), target_destination + "/" + File.basename(dll), force: true
  end
end
