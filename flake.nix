{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config = {
        android_sdk.accept_license = true;
        allowUnfree = true;
      };
    };

    pinnedJDK = pkgs.jdk17;
    buildToolsVersion = "36.0.0";
    ndkVersion = "27.1.12297006";
    androidComposition = pkgs.androidenv.composeAndroidPackages {
      cmdLineToolsVersion = "19.0"; # CLI tools
      toolsVersion = "26.1.1"; # Legacy tools
      platformToolsVersion = "36.0.0"; # Platform tools
      buildToolsVersions = [buildToolsVersion "35.0.0" "34.0.0"];
      includeEmulator = true;
      emulatorVersion = "36.1.2";
      # Target Android version
      platformVersions = ["36"];
      includeSources = false;
      includeSystemImages = true;
      systemImageTypes = ["google_apis_playstore"];
      abiVersions = ["x86_64" "arm64-v8a"];
      cmakeVersions = ["3.10.2" "3.22.1"];
      includeNDK = true;
      ndkVersion = ndkVersion;
      useGoogleAPIs = false;
      useGoogleTVAddOns = false;
      includeExtras = [
        "extras;google;gcm"
      ];
    };

    sdk = androidComposition.androidsdk;
  in {
    devShells.${system} = {
      shell = pkgs.mkShell rec {
        buildInputs = with pkgs; [
          pinnedJDK
          sdk
          pkg-config
          eas-cli
          go
          bun
          nodejs
          gst_all_1.gstreamer
          gst_all_1.gst-plugins-base
          gst_all_1.gst-plugins-good
          gst_all_1.gst-plugins-bad
        ];
        ANDROID_SDK_ROOT = "${androidComposition.androidsdk}/libexec/android-sdk";
        ANDROID_NDK_HOME = "${ANDROID_SDK_ROOT}/ndk/${ndkVersion}";
        GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${ANDROID_SDK_ROOT}/build-tools/${buildToolsVersion}/aapt2";

        GST_PLUGIN_SYSTEM_PATH_1_0="${pkgs.gst_all_1.gst-plugins-base}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-good}/lib/gstreamer-1.0:${pkgs.gst_all_1.gst-plugins-bad}/lib/gstreamer-1.0";
      };
      default = self.devShells.${system}.shell;
    };
  };
}
