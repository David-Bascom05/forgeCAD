#!/usr/bin/env bash
# Fetch the makers' Fusion tool libraries hosted by Autodesk ("Tools for Autodesk Fusion",
# https://cam.autodesk.com/hsmtools) into a git-ignored cache.
#
# TERMS: the libraries carry no licence of their own; Autodesk's website terms make downloads
# "available for downloading, provided that the materials are (a) used for your personal,
# informational, non-commercial purposes only; (b) not modified or altered in any manner; and
# (c) not redistributed" (https://www.autodesk.com/company/legal-notices-trademarks/website-terms-of-use,
# quoted 2026-11-24 from a search extract: autodesk.com refused direct fetches). David's decision
# (2026-11-24): use them, never commit them. Commit NOTHING from them: no files, no records, no
# per-tool derived values; only our own code and aggregate results. They serve to cross-check our
# PDF-cited records and, at runtime behind an opt-in (FORGECAD_TOOL_LIBRARIES), to benchmark.
# tools/ci/no-vendor-tool-libraries.py fails CI if any of them is committed.
# research/cam/tooling/libraries.md lists every library, its fields and terms.
#
# Usage: tools/fetch-tool-libraries.sh [NAME_SUBSTRING ...] [--dest DIR]
#   no NAME: all 45 Fusion JSON libraries (~690 MB); else those whose file name contains one.
# Default destination: $FORGECAD_TOOL_LIBRARIES, else <repo>/.cache/tool-libraries (git-ignored).
# Each file is verified against the SHA-256 pinned below (downloaded 2026-11-24) and refused on a
# mismatch: a library the maker updated must be re-pinned on purpose.
set -euo pipefail

# file|url|bytes|sha256
LIBS=(
  "Harvey_Tool_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=Harvey%20Tool/End%20Mills|37731687|c11d541e6a652869a5d8fb6e0900f7223110a99f3bbcbca89e75c81d8639e618"
  "Harvey_Tool_Holemaking_and_Threading.json|https://cam.autodesk.com/tools/download.php?name=Harvey%20Tool/Holemaking%20and%20Threading|14113221|18f9dae93afac7a6e840c04f3d4b8fb2720ddd38bc04583dc368217d9f251a60"
  "Harvey_Tool_Specialty_Profiles.json|https://cam.autodesk.com/tools/download.php?name=Harvey%20Tool/Specialty%20Profiles|25285681|a7dbff285dcecf46476d97c876bca3ee2f9cee7e5981539042a97f3c18f6a480"
  "Helical_Solutions_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=Helical%20Solutions/End%20Mills|17897822|cb4127cdd9109dd2d822dc383d692d258eaccbae7dad1fb51e7f730db11dcf1a"
  "Helical_Solutions_Specialty_Profiles.json|https://cam.autodesk.com/tools/download.php?name=Helical%20Solutions/Specialty%20Profiles|558347|60e60ca243603cf6f586262c6ca2cbc932ea3cd305638344b5dfb982639c56cf"
  "BitsBits_BitsBits_Cutting_Tools.json|https://cam.autodesk.com/tools/download.php?name=BitsBits/BitsBits%20Cutting%20Tools|319228|6e88e61f2e2d69c71ecbff6be703d001a514667285da926aa61b953fa6263b2c"
  "IMCO_Solid_End_Mills_Metric.json|https://cam.autodesk.com/tools/download.php?name=IMCO/Solid%20End%20Mills%20Metric|55661102|e5f7cd261df00d94343a3a542a17ebd2b8366d7bf91545d993a3ecfb5ad6054d"
  "IMCO_Solid_End_Mills_Imperial.json|https://cam.autodesk.com/tools/download.php?name=IMCO/Solid%20End%20Mills%20Imperial|105678085|d22e93ecd2fc0a7bf50013a3b352419d6f094104e1bb17ff500c16f03a443e42"
  "SGS_Kyocera_SGS_Carbide_Tools_Metric.json|https://cam.autodesk.com/tools/download.php?name=SGS%20Kyocera/SGS%20Carbide%20Tools%20Metric|12428019|ef4ca64299a52408b5a69126870e425ac59b6dae0ef574f9af5e7d859ac0b9ab"
  "SGS_Kyocera_SGS_Carbide_Tools_Imperial.json|https://cam.autodesk.com/tools/download.php?name=SGS%20Kyocera/SGS%20Carbide%20Tools%20Imperial|18065102|8507c19605668a0a92a482823ed6c742d53634bf338654f043307cc767e5c98a"
  "Haas_Tooling_Haas_Tooling.json|https://cam.autodesk.com/tools/download.php?name=Haas%20Tooling/Haas%20Tooling|11724573|14f7c08e612de9ecf678aa903597f4d4c916fa4b06b6051c84064788015d99c6"
  "Haas_Tooling_Haas_Holders.json|https://cam.autodesk.com/tools/download.php?name=Haas%20Tooling/Haas%20Holders|1743259|5fa90b2d10fc8e7e63279a47c32536f3b7f47f29d1b344e847ffada53cb460e4"
  "Haimer_Haimer_Holders.json|https://cam.autodesk.com/tools/download.php?name=Haimer/Haimer%20Holders|2124456|cbaae3d16094e7a3fb7ebb4f04e21d3566043c9a891b79ecf44b17360fc06f87"
  "SwiftCARB_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=SwiftCARB/End%20Mills|57059218|91eed6bdf508103eb8963feb9b280dee2a81b0da60e5cd441b5b6a55d95774bb"
  "DATRON_DATRON_Cutting_Tools.json|https://cam.autodesk.com/tools/download.php?name=DATRON/DATRON%20Cutting%20Tools|2416049|45a14b0caab853a30e973058a47c5e5f58eb3d085a52cb5c4658dffb1000b64f"
  "Delta_Tools_HPC_Endmill_Solid_Carbide.json|https://cam.autodesk.com/tools/download.php?name=Delta%20Tools/HPC%20Endmill%20Solid%20Carbide|2081434|e1ed67605cdc75457febb805b58086fd15d55fb34ec3bea494e5bac2ba43409f"
  "Destiny_Tool_Destiny_Tool.json|https://cam.autodesk.com/tools/download.php?name=Destiny%20Tool/Destiny%20Tool|4647874|e52a8479d0d5e84f864844057e6f05b2e6cb9043f949803b6043e6e6994ab952"
  "Renishaw_Renishaw___OMP_Probes.json|https://cam.autodesk.com/tools/download.php?name=Renishaw/Renishaw%20-%20OMP%20Probes|67461|4fe5898f2e7c31974f24ed4e319ac27067991762601efb1f0fdaad3063abdf3c"
  "Blum_Blum_Probes.json|https://cam.autodesk.com/tools/download.php?name=Blum/Blum%20Probes|112808|14764db4b81c4dddb261e61f9aa7b37c70f03d39cc7b2aca4efe2a3b688e3e0d"
  "Tormach_Tormach___15L_Slant_PRO___Starter.json|https://cam.autodesk.com/tools/download.php?name=Tormach/Tormach%20-%2015L%20Slant-PRO%20-%20Starter|38730|9864a045e3d58627d0a97d82346bb34a856f228f44a7c94163348bc8390ca9c5"
  "Tormach_Tormach___PCNC_440__PCNC_770___Starter.json|https://cam.autodesk.com/tools/download.php?name=Tormach/Tormach%20-%20PCNC%20440%2C%20PCNC%20770%20-%20Starter|76878|2c441279ff9d5ac166e01a85e32c797170717b7d5654b9affffd7900160405db"
  "Tormach_Tormach___PCNC_1100___Starter.json|https://cam.autodesk.com/tools/download.php?name=Tormach/Tormach%20-%20PCNC%201100%20-%20Starter|76803|96a37793478576f7773d3d19f0caf86c0ea65323a70fe2bed748bcde22824eb2"
  "Sandvik_Coromant_Solid_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=Sandvik%20Coromant/Solid%20End%20Mills|6986529|1d4d575f262d666aface5edb5e483be2770161af3b65dda70719ab5d41ddaf30"
  "Sandvik_Coromant_Solid_Hole_Making.json|https://cam.autodesk.com/tools/download.php?name=Sandvik%20Coromant/Solid%20Hole%20Making|8513904|d4ea3776c4eb4ac8e02fdf890fd3eb195bbd4f3967640a26c3d0231d5bacbb67"
  "Guhring_Solid_Hole_Making.json|https://cam.autodesk.com/tools/download.php?name=Guhring/Solid%20Hole%20Making|53283701|2486f47c10a21768c5efbcfed4367a2b767ca3c14c2c64d4b76eb61066db1979"
  "NexGen_Tooling_NexGen_Tooling.json|https://cam.autodesk.com/tools/download.php?name=NexGen%20Tooling/NexGen%20Tooling|9384048|8ac3b0b7d7d37dd0a0859b74c31d76e2bbe870dc90263acc1ec743076a09f392"
  "Garr_Tool_Garr_Tool.json|https://cam.autodesk.com/tools/download.php?name=Garr%20Tool/Garr%20Tool|27209663|556996db2556852ceba810396956e35fe2effa33112297989acc6f746a65415e"
  "Valor_Holemaking_Valor_Holemaking.json|https://cam.autodesk.com/tools/download.php?name=Valor%20Holemaking/Valor%20Holemaking|7811679|2f98bfd4d2c0ea523ba8cbb90892fd46d6168943a851eea2caf1d7aac93e46c5"
  "OSG_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=OSG/End%20Mills|3760451|9971d0b5804a3954c04aef67c9ff152cbf56cc5ae592c104aec9575ec475b385"
  "Iscar_Iscar_Hole_Making_Imperial.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Iscar%20Hole%20Making%20Imperial|735417|714112c2f606238fb61594871e56b3f5b13980f3e21bcf96f38a1000e3d9a3ae"
  "Iscar_Iscar_Hole_Making_Metric.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Iscar%20Hole%20Making%20Metric|2917571|6540d3f2393a3a35eabf52281d2c2dbd817fafb82137308ff8a3958817d91f05"
  "Iscar_Iscar_Solid_End_Mills_Imperial.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Iscar%20Solid%20End%20Mills%20Imperial|8705425|a3a799c9b51219e49072d84cc649a999a43f2af018550e1d584796a923d19382"
  "Iscar_Iscar_Solid_End_Mills_Metric.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Iscar%20Solid%20End%20Mills%20Metric|7088991|17a2458177e1a89f93f8ecb48f88e64177b1e1622fa4eef496a88de2175d5952"
  "Fraiser_Tools_Boss_Beaver.json|https://cam.autodesk.com/tools/download.php?name=Fraiser%20Tools/Boss%20Beaver|304725|80d51ff3e504b4ad41aafd2460311e0b7433bacbb370a7cb4bf94698e93ed3a3"
  "Fraiser_Tools_Carbider_Poly.json|https://cam.autodesk.com/tools/download.php?name=Fraiser%20Tools/Carbider%20Poly|292108|47c395568356f8007659151875ac1ac36e3d5169056f6a9ba0a9ad1b33230cb8"
  "XEBEC_XEBEC_Deburring_tools.json|https://cam.autodesk.com/tools/download.php?name=XEBEC/XEBEC%20Deburring%20tools|121831|0795f15498e26f5b1287722630ffe024215c0e4ed29e121fd0f2b722b678e01a"
  "Schunk_Schunk_Holders.json|https://cam.autodesk.com/tools/download.php?name=Schunk/Schunk%20Holders|4820929|167fc813b32bc5a0af85392efa3b32667970c5032a1e7278222eff59ff3d84b0"
  "Kennametal_Turning_Imperial_Turning_Imperial.json|https://cam.autodesk.com/tools/download.php?name=Kennametal/Turning%20Imperial/Turning%20Imperial|145098|116826eeacf484498e081ed05a1b2a540d2b6378127c59d56f204e24fad7be6f"
  "MA_Ford_MA_Ford_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=MA%20Ford/MA%20Ford%20End%20Mills|47065|0a7954e79468919756861826a4642b5f5a9fddb5f4971fe53ecbd331c6913cbe"
  "MA_Ford_Europe_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=MA%20Ford%20Europe/End%20Mills|516599|f682ed3145333e705d0f2ae78f4617210886de972f8400c66fc6a7795f9d807d"
  "Sandvik_Coromant_Turning_Imperial_Turning_Imperial.json|https://cam.autodesk.com/tools/download.php?name=Sandvik%20Coromant/Turning%20Imperial/Turning%20Imperial|138249|8e20ed08ee6228823c794aabed2795b1fc94ea879de5257dc05ba3df3f0a418d"
  "Sandvik_Coromant_Turning_Metric_Turning_Metric.json|https://cam.autodesk.com/tools/download.php?name=Sandvik%20Coromant/Turning%20Metric/Turning%20Metric|624030|926502ff6e12d92b8d2282373cec5b9cd59478eb559cd72370044780e3e35464"
  "Iscar_Turning_Imperial_Turning_Imperial.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Turning%20Imperial/Turning%20Imperial|181667|78fdfe217aeec295239bc5351e079a624cb3405003e3e705773745b2b3594430"
  "Iscar_Turning_Metric_Turning_Metric.json|https://cam.autodesk.com/tools/download.php?name=Iscar/Turning%20Metric/Turning%20Metric|270530|3491a78f013ac0e4f024251bb077b99d474e85ff67c0fb927d5860272e902134"
  "Mitsubishi_Materials_Solid_End_Mills.json|https://cam.autodesk.com/tools/download.php?name=Mitsubishi%20Materials/Solid%20End%20Mills|45092721|f2c7cdff20c48a843704fe457435f8042c578da62bef3d0ad201435bbe63bc66"
)

repo=$(cd "$(dirname "$0")/.." && pwd)
dest=${FORGECAD_TOOL_LIBRARIES:-$repo/.cache/tool-libraries}
want=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dest) dest=$2; shift 2 ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) want+=("$1"); shift ;;
  esac
done

case "$dest" in
  "$repo"/*)
    mkdir -p "$dest"
    if ! git -C "$repo" check-ignore -q "$dest/x"; then
      echo "refused: $dest is inside the repository and not git-ignored" >&2; exit 1
    fi ;;
esac
mkdir -p "$dest"
cat > "$dest/NOTICE.txt" <<NOTE
Makers' tool libraries from Autodesk's "Tools for Autodesk Fusion" (https://cam.autodesk.com/hsmtools).
Autodesk website terms: personal, informational, non-commercial use; not modified; not redistributed.
This cache must never be committed to the forgeCAD repository (tools/fetch-tool-libraries.sh).
NOTE

for entry in "${LIBS[@]}"; do
  IFS='|' read -r file url bytes sha <<<"$entry"
  if [ ${#want[@]} -gt 0 ]; then
    hit=0; for w in "${want[@]}"; do case "$file" in *"$w"*) hit=1 ;; esac; done
    [ $hit = 1 ] || continue
  fi
  out="$dest/$file"
  if [ ! -f "$out" ] || [ "$(stat -c %s "$out")" != "$bytes" ]; then
    echo "fetching $file ($bytes bytes)"
    curl -fSL --retry 4 "$url" -o "$out.part" && mv "$out.part" "$out"
  fi
  got=$(sha256sum "$out" | cut -d' ' -f1)
  if [ "$got" != "$sha" ]; then
    echo "refused: $file sha256 $got, pinned $sha (the maker may have updated it: re-pin on purpose)" >&2
    exit 1
  fi
  echo "$file: ok"
done
echo "cache: $dest"
