import json
import time
from typing import TypeAlias

from lib.build_spell_html import build_spell_html
from lib.compute_spell_variables import compute_spell_variables
from lib.utils import (
    CDRAGON_URL,
    DATA_DIR,
    GUI_ASSETS_DIR,
    LATEST_SET_ID,
    LATEST_SET_NUMBER,
    LATEST_SET_PREFIX,
    download_image,
    fetch_json_cached,
    get_cdragon_asset_url,
)

USE_CACHED = False

###

TEAM_PLANNER_DATA_URL = (
    CDRAGON_URL
    / "plugins/rcp-be-lol-game-data/global/default/v1/tftchampions-teamplanner.json"
)
TEAM_PLANNER_DATA_FILE = DATA_DIR / "tftchampions-teamplanner.json"

CHARACTERS_DATA_URL = CDRAGON_URL / "game/data/tftteamplanner/characters.bin.json"
CHARACTERS_DATA_FILE = DATA_DIR / "characters.bin.json"

SET_DATA_URL = CDRAGON_URL / "cdragon/tft/en_us.json"
SET_DATA_FILE = DATA_DIR / "en_us.json"

MERGED_DATA_FILE = GUI_ASSETS_DIR / "tft" / "merged_teamplanner_data.json"

ICON_DIR = GUI_ASSETS_DIR / "tft" / "champions"
ICON_DIR.mkdir(parents=True, exist_ok=True)

ABILITY_ICON_DIR = GUI_ASSETS_DIR / "tft" / "abilities"
ABILITY_ICON_DIR.mkdir(parents=True, exist_ok=True)

SPLASH_DIR = GUI_ASSETS_DIR / "tft" / "champion_splashes"
SPLASH_DIR.mkdir(parents=True, exist_ok=True)

CHARACTER_BIN_DIR = DATA_DIR / "character_bins"
CHARACTER_BIN_DIR.mkdir(parents=True, exist_ok=True)

STRINGS_URL = (
    CDRAGON_URL / "game" / "en_us" / "data" / "menu" / "en_us" / "main.stringtable.json"
)
STRINGS_FILE = DATA_DIR / "main.stringtable.json"

###

ITeamPlannerData: TypeAlias = list[dict]
IStringData: TypeAlias = dict
ICharactersData: TypeAlias = dict
ISetData: TypeAlias = dict
ICharacterBinData: TypeAlias = dict[str, dict]
IMergedData: TypeAlias = list[dict]


def fetch_teamplanner_data() -> ITeamPlannerData:
    return fetch_json_cached(
        TEAM_PLANNER_DATA_URL,
        TEAM_PLANNER_DATA_FILE,
        use_cache=USE_CACHED,
    )


def fetch_string_data() -> IStringData:
    return fetch_json_cached(
        STRINGS_URL,
        STRINGS_FILE,
        use_cache=USE_CACHED,
    )


def fetch_characters_data() -> ICharactersData:
    return fetch_json_cached(
        CHARACTERS_DATA_URL,
        CHARACTERS_DATA_FILE,
        use_cache=USE_CACHED,
    )


def fetch_character_bin_data(id: str) -> ICharacterBinData:
    file_name = id.lower() + ".cdtb.bin.json"
    url = CDRAGON_URL / "game" / "characters" / file_name
    file = CHARACTER_BIN_DIR / file_name
    return fetch_json_cached(url, file)


def fetch_set_data() -> ICharactersData:
    return fetch_json_cached(
        SET_DATA_URL,
        SET_DATA_FILE,
        use_cache=USE_CACHED,
    )


def build_merged_data(
    tp_data: ITeamPlannerData,
    string_data: IStringData,
    char_data: ICharactersData,
    bin_data: ICharacterBinData,
    set_data: ISetData,
) -> IMergedData:
    """
    Append the stats (ad, range, etc) from set data to the team planner data for each champion
    """

    latest_set = next(v for k, v in set_data["sets"].items() if k == LATEST_SET_NUMBER)

    merged: IMergedData = []
    for c in tp_data:
        role = find_role_for_champion(c["character_id"], char_data)

        character_bin = bin_data[c["character_id"]]

        stats = next(v for k, v in character_bin.items() if k.endswith("/Root"))
        scripts = [v for k, v in character_bin.items() if k.endswith("Spell")]
        assert len(scripts) == 1

        set_champion = next(
            x for x in latest_set["champions"] if x["apiName"] == c["character_id"]
        )

        variables = compute_spell_variables(scripts[0]["mSpell"], stats)

        spell = (
            f'generatedtip_spelltft_{c["character_id"].lower()}spell_tooltipextended'
        )
        spell_tooltip = string_data["entries"][spell]
        spell_tooltip_html = build_spell_html(spell_tooltip, variables)

        merged.append(
            dict(
                # from teamplanner data
                character_id=c["character_id"],
                tier=c["tier"],
                display_name=c["display_name"],
                traits=c["traits"],
                # from string + bin data
                spell=spell_tooltip_html,
                # from character data
                damage_type=get_damage_type(role),
                # from set data
                stats=set_champion["stats"],
            )
        )

    return merged


def find_role_for_champion(id: str, char_data: ICharactersData) -> dict:
    char = next(c for c in char_data.values() if c.get("mCharacterName") == id)
    role = next(
        data for name, data in char_data.items() if name == char["CharacterRole"]
    )
    return role


def get_damage_type(role: dict) -> dict:
    if role["name"].startswith("AP"):
        return dict(
            is_ad=False,
            is_ap=True,
        )
    elif role["name"].startswith("AD"):
        return dict(
            is_ad=True,
            is_ap=False,
        )
    else:
        raise Exception(role["name"])


# def infer_damage_type(set_data):
#     is_ad = "<physicalDamage>" in set_data["ability"]["desc"]
#     is_ap = "<magicDamage>" in set_data["ability"]["desc"]

#     return dict(is_ad=is_ad, is_ap=is_ap,)


def download_icons(data: ITeamPlannerData):
    for champion in data:
        url = get_cdragon_asset_url(champion["squareIconPath"])
        ext = str(url).split(".")[-1]

        id = champion["character_id"]
        fp_out = ICON_DIR / f"{id}.{ext}"
        if fp_out.exists():
            continue

        print("Downloading icon", url)
        download_image(url, fp_out)
        time.sleep(1)


def download_ability_icons(data: ITeamPlannerData):
    for champion in data:
        id = champion["character_id"]

        url = (
            CDRAGON_URL
            / "game"
            / "assets"
            / "characters"
            / id.lower()
            / "hud"
            / "icons2d"
            / f"{id.lower()}_ability.png"
        )

        if id == "TFT11_FortuneYord":
            url = str(url).split("/")
            url[-1] = f"tft11_fortuneyord_ability.tft_set11.png"
            url = "/".join(url)
        elif id == "TFT11_Azir":
            url = str(url).split("/")
            url[-1] = "tft11_azir_passive.tft_set11.png"
            url = "/".join(url)
        elif id == "TFT11_Nautilus":
            url = str(url).split("/")
            url.remove("icons2d")
            url[-1] = "tft11_nautilus_r.tft_set11.png"
            url = "/".join(url)

        ext = str(url).split(".")[-1]

        fp_out = ABILITY_ICON_DIR / f"{id}.{ext}"
        if fp_out.exists():
            continue

        print("Downloading ability icon", url)
        download_image(url, fp_out)
        time.sleep(1)


def download_splashes(data: ITeamPlannerData):
    for champion in data:
        url = get_cdragon_asset_url(champion["squareSplashIconPath"])
        ext = str(url).split(".")[-1]

        id = champion["character_id"]
        fp_out = SPLASH_DIR / f"{id}.{ext}"
        if fp_out.exists():
            continue

        print("Downloading splash", url)
        download_image(url, fp_out)
        time.sleep(1)


def main():
    tp_data = fetch_teamplanner_data()
    tp_data_filtered = [d for d in tp_data if LATEST_SET_PREFIX in d["character_id"]]

    string_data = fetch_string_data()
    char_data = fetch_characters_data()
    bin_data = {
        c["character_id"]: fetch_character_bin_data(c["character_id"])
        for c in tp_data_filtered
    }
    set_data = fetch_set_data()

    merged = build_merged_data(
        tp_data_filtered,
        string_data,
        char_data,
        bin_data,
        set_data,
    )
    print(f"Found {len(merged)} champions for set {LATEST_SET_ID}")

    download_icons(tp_data_filtered)
    download_ability_icons(tp_data_filtered)
    download_splashes(tp_data_filtered)

    print("Creating", MERGED_DATA_FILE)
    with open(MERGED_DATA_FILE, "w+") as file:
        json.dump(merged, file, indent=4)


main()
