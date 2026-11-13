# Like Square on the same VPS, package a native release built outside the Nix
# sandbox: Trunk downloads wasm-bindgen and wasm-opt during its build.
# autoPatchelfHook switches the Linux executable to Nix's loader and libraries.
# Only the server binary is the source, so website-only changes keep this store
# path and do not restart the in-memory game server; web.nix packages the site.
{ lib, stdenv, autoPatchelfHook }:
stdenv.mkDerivation {
  pname = "eins";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./bundle;
    fileset = ./bundle/eins_server;
  };
  nativeBuildInputs = [ autoPatchelfHook ];
  buildInputs = [ stdenv.cc.cc.lib ];
  dontConfigure = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp eins_server $out/bin/
    runHook postInstall
  '';
}
