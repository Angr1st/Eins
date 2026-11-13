# The compiled website served by eins-web. It is packaged separately from the
# game server, so a website-only update restarts only Nginx, not eins.
{ stdenvNoCC }:
stdenvNoCC.mkDerivation {
  pname = "eins-web";
  version = "0.1.0";
  src = ./bundle/public;
  dontConfigure = true;
  dontBuild = true;
  installPhase = ''
    runHook preInstall
    mkdir -p $out/share/eins
    cp -r . $out/share/eins/
    runHook postInstall
  '';
}
