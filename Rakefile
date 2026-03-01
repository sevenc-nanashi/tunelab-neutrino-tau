# frozen_string_literal: true

require "fileutils"
require "tmpdir"

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

task :pack do
  project_file = File.join(__dir__, "NeutrinoTau.csproj")
  release_dir = File.join(__dir__, "bin", "Release", "net8.0")
  artifacts_dir = File.join(__dir__, "artifacts")
  package_name = ENV.fetch("PACKAGE_NAME", "tunelab-neutrino-tau")
  zip_path = File.join(artifacts_dir, "#{package_name}.zip")
  tlx_path = File.join(artifacts_dir, "#{package_name}.tlx")
  staging_dir = Dir.mktmpdir("./pack.stage", __dir__)

  begin
    sh "cargo build"
    sh "dotnet build \"#{project_file}\" -c Release"

    mkdir_p artifacts_dir
    cp File.join(__dir__, "description.json"), File.join(staging_dir, "description.json")

    dlls = Dir.glob(File.join(release_dir, "*.dll"))
    raise "No DLL found in #{release_dir}" if dlls.empty?

    dlls.each do |dll|
      cp dll, File.join(staging_dir, File.basename(dll))
    end

    escaped_zip_path = zip_path.gsub("'", "''")
    escaped_staging_dir = staging_dir.gsub("'", "''")
    sh %(pwsh -NoLogo -NoProfile -Command "if (Test-Path -LiteralPath '#{escaped_zip_path}') { Remove-Item -LiteralPath '#{escaped_zip_path}' -Force }; Compress-Archive -Path '#{escaped_staging_dir}/*' -DestinationPath '#{escaped_zip_path}' -Force")
    mv zip_path, tlx_path, force: true

    puts "Packed: #{tlx_path}"
  ensure
    FileUtils.remove_entry(staging_dir, true) if Dir.exist?(staging_dir)
  end
end
