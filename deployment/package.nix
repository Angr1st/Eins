# Like Square on the same VPS, package a native release built outside the Nix
# sandbox: Trunk downloads wasm-bindgen and wasm-opt during its build.
# autoPatchelfHook switches the Linux executable to Nix's loader and libraries.
{ stdenv, autoPatchelfHook }:
stdenv.mkDerivation {
  pname = "eins";
  version = "0.1.0";
  src = ./bundle;
  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib ];
  dontConfigure = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/share/eins
    cp eins_server $out/bin/
    cp -r public/. $out/share/eins/
    runHook postInstall
  '';
}
