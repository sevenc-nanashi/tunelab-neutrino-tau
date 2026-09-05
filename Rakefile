# frozen_string_literal: true

require "fileutils"
require "tmpdir"

task :link do
  target_destination = ENV["APPDATA"] + "/TuneLab/Extensions/Neutrino Tau"
  # output_dir = "#{__dir__}/bin/Debug/net8.0"
  # nuget_dir = ENV["USERPROFILE"] + "/.nuget/packages"
  mkdir_p target_destination
  ln_s "#{__dir__}/src/manifest.json", target_destination + "/manifest.json", force: true
  ln_s "#{__dir__}/src/docs/introduction.en.md", target_destination + "/introduction.en.md", force: true
  ln_s "#{__dir__}/src/docs/introduction.ja.md", target_destination + "/introduction.ja.md", force: true
  ["NeutrinoTau.dll", "neutrino_tau_native.dll"].each do |name|
    dll = File.join(__dir__, "bin", "Debug", "net8.0", name)
    raise "Missing build output: #{dll}" unless File.file?(dll)

    ln_s File.expand_path(dll), target_destination + "/" + File.basename(dll), force: true
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
    # NativeMethods.g.csを生成するために一旦cargo buildする、これはdebug buildで十分
    sh "cargo build"
    sh "dotnet build \"#{project_file}\" -c Release"

    mkdir_p artifacts_dir
    cp File.join(__dir__, "src/manifest.json"), File.join(staging_dir, "manifest.json")
    cp File.join(__dir__, "src/docs/introduction.en.md"), File.join(staging_dir, "introduction.en.md")
    cp File.join(__dir__, "src/docs/introduction.ja.md"), File.join(staging_dir, "introduction.ja.md")

    dlls = ["NeutrinoTau.dll", "neutrino_tau_native.dll"].map do |name|
      path = File.join(release_dir, name)
      raise "Missing build output: #{path}" unless File.file?(path)

      path
    end

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
