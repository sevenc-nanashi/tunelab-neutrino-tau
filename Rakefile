# frozen_string_literal: true

require "fileutils"
require "rbconfig"
require "tmpdir"

task :link do
  raise "link task is only supported on Windows" unless windows?

  target_destination = ENV["APPDATA"] + "/TuneLab/Extensions/Neutrino Tau"
  # output_dir = "#{__dir__}/bin/Debug/net8.0"
  # nuget_dir = ENV["USERPROFILE"] + "/.nuget/packages"
  mkdir_p target_destination
  ln_s "#{__dir__}/description.json", target_destination + "/description.json", force: true
  Dir.glob("./bin/Debug/net8.0/*.{dll,dylib,so}").each do |library|
    ln_s File.expand_path(library), target_destination + "/" + File.basename(library), force: true
  end
end

task :pack do
  project_file = File.join(__dir__, "NeutrinoTau.csproj")
  release_dir = File.join(__dir__, "bin", "Release", "net8.0")
  artifacts_dir = File.join(__dir__, "artifacts")
  package_name = ENV.fetch("PACKAGE_NAME", "Neutrino Tau")
  zip_path = File.join(artifacts_dir, "#{package_name}.zip")
  tlx_path = File.join(artifacts_dir, "#{package_name}.tlx")
  staging_dir = Dir.mktmpdir("./pack.stage", __dir__)

  begin
    sh "cargo build"
    sh "dotnet build \"#{project_file}\" -c Release"

    mkdir_p artifacts_dir
    cp File.join(__dir__, "description.json"), File.join(staging_dir, "description.json")

    libraries = Dir.glob(File.join(release_dir, "*.{dll,dylib,so}"))
    raise "No library found in #{release_dir}" if libraries.empty?

    libraries.each do |library|
      cp library, File.join(staging_dir, File.basename(library))
    end

    rm_f zip_path
    if windows?
      escaped_zip_path = zip_path.gsub("'", "''")
      escaped_staging_dir = staging_dir.gsub("'", "''")
      sh %(pwsh -NoLogo -NoProfile -Command "Compress-Archive -Path '#{escaped_staging_dir}/*' -DestinationPath '#{escaped_zip_path}' -Force")
    else
      sh "cd \"#{staging_dir}\" && zip -qr \"#{zip_path}\" ."
    end
    mv zip_path, tlx_path, force: true

    puts "Packed: #{tlx_path}"
  ensure
    FileUtils.remove_entry(staging_dir, true) if Dir.exist?(staging_dir)
  end
end

def windows?
  RbConfig::CONFIG["host_os"].match?(/mswin|mingw|cygwin/)
end
