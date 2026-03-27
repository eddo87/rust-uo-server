/// Loads NPC spawn points from a ServUO-style `felucca.xml` file
/// and populates an `NpcManager`.
///
/// Only `<Points>` entries with `<IsRunning>True</IsRunning>` are loaded.
/// Each entry may declare multiple NPC types in `<Objects2>`; we spawn one
/// representative NPC per type at the point's centre coordinates.
use crate::combat::ArmorStats;
use crate::movement::Position;
use crate::npc::{Npc, NpcManager};
use quick_xml::events::Event;
use quick_xml::Reader;

// ---------------------------------------------------------------------------
// NPC template table
// ---------------------------------------------------------------------------

struct NpcTemplate {
    body_type: u16,
    hue: u16,
    hit_points: i32,
    defense_skill: f32,
    notoriety: u8,
    armor: ArmorStats,
}

/// Return the display/combat template for a ServUO class name.
/// Returns `None` for non-mob entries (treasure chests, teleporters, etc.).
fn template_for(class_name: &str) -> Option<NpcTemplate> {
    let lower = class_name.to_lowercase();

    // Non-mob objects – skip entirely.
    if lower.starts_with("treasure")
        || lower.contains("fieldtele")
        || lower.contains("waters")
        || lower.contains("journal")
        || lower.contains("bauble")
        || lower.contains("section")
        || lower.contains("spawn")
        || lower.contains("gate")
        || lower.contains("portal")
        || lower.contains("chest")
    {
        return None;
    }

    Some(match lower.as_str() {
        // ----------------------------------------------------------------
        // Dragons / wyrms
        // ----------------------------------------------------------------
        "dragon" | "ancientwyrm" | "greaterdragon" => tmpl(0x00C9, 2000, 70.0, 0x06, 40, 100, 20, 40, 40),
        "drake" => tmpl(0x00C6, 400, 50.0, 0x06, 20, 60, 10, 20, 20),

        // ----------------------------------------------------------------
        // Humanoid monsters
        // ----------------------------------------------------------------
        "orc" | "orcmage" | "orclord" | "orcbomber" | "orciscout" | "orcisspy" => tmpl(0x0011, 70, 30.0, 0x06, 15, 0, 0, 0, 0),
        "ettin" => tmpl(0x0012, 200, 45.0, 0x06, 10, 0, 0, 0, 0),
        "cyclops" => tmpl(0x001D, 350, 55.0, 0x06, 20, 0, 0, 0, 0),
        "gargoyle" | "stonegargoyle" | "gargoylewarrior" => tmpl(0x002F, 200, 45.0, 0x06, 20, 0, 0, 0, 0),
        "troll" => tmpl(0x0036, 150, 40.0, 0x06, 15, 0, 0, 0, 0),
        "ratman" | "ratmage" | "ratmancer" => tmpl(0x002B, 90, 35.0, 0x06, 10, 0, 0, 0, 0),
        "lizardman" | "lizardmanshaman" => tmpl(0x0024, 80, 30.0, 0x06, 10, 0, 0, 0, 0),

        // ----------------------------------------------------------------
        // Undead
        // ----------------------------------------------------------------
        "skeleton" | "boneknight" | "bonemagi" | "bonekight" => tmpl(0x0032, 45, 25.0, 0x06, 10, 0, 30, 20, 10),
        "zombie" => tmpl(0x000E, 45, 20.0, 0x06, 5, 0, 30, 10, 0),
        "ghoul" => tmpl(0x000F, 60, 25.0, 0x06, 5, 0, 30, 10, 0),

        // ----------------------------------------------------------------
        // Daemons
        // ----------------------------------------------------------------
        "daemon" => tmpl(0x0048, 1000, 70.0, 0x06, 35, 100, 20, 20, 20),
        "balron" => tmpl(0x0049, 1500, 75.0, 0x06, 40, 100, 25, 25, 25),

        // ----------------------------------------------------------------
        // Elementals
        // ----------------------------------------------------------------
        "airelemental" | "earthelemental" | "waterelemental" | "fireelemental"
        | "bloodelemental" | "acidelemental" | "dullcopperelemental"
        | "crystalvortex" => {
            tmpl(0x0014, 250, 50.0, 0x06, 20, 20, 20, 20, 20)
        }

        // ----------------------------------------------------------------
        // Canines / felines / wildlife
        // ----------------------------------------------------------------
        "timberwolf" | "greywolf" | "direwolf" => tmpl(0x001A, 60, 30.0, 0x06, 5, 0, 0, 0, 0),
        "blackbear" | "brownbear" | "grizzlybear" | "polarbear" => tmpl(0x00D5, 80, 30.0, 0x06, 5, 0, 0, 0, 0),
        "cougar" | "panther" | "cat" => tmpl(0x00DB, 15, 10.0, 0x03, 0, 0, 0, 0, 0),
        "dog" => tmpl(0x00DB, 15, 10.0, 0x03, 0, 0, 0, 0, 0),

        // ----------------------------------------------------------------
        // Livestock / passive animals
        // ----------------------------------------------------------------
        "horse" | "hind" | "greathart" | "greaterhart" | "sheep" | "goat"
        | "pig" | "cow" | "bull" | "boar" => {
            tmpl(0x00C8, 25, 10.0, 0x03, 0, 0, 0, 0, 0)
        }
        "bird" | "eagle" | "chicken" => tmpl(0x00D0, 5, 5.0, 0x03, 0, 0, 0, 0, 0),
        "rat" => tmpl(0x00EE, 5, 5.0, 0x06, 0, 0, 0, 0, 0),
        "dolphin" => tmpl(0x0097, 25, 10.0, 0x03, 0, 0, 0, 0, 0),
        "alligator" | "desertostard" => tmpl(0x00DA, 100, 35.0, 0x06, 10, 0, 0, 0, 0),

        // ----------------------------------------------------------------
        // Arthropods / plants
        // ----------------------------------------------------------------
        "giantspider" | "dreadspider" | "terathanhatchling" | "terathanwarrior"
        | "terathanmatriarch" | "bogling" | "bogthing" => {
            tmpl(0x001E, 80, 30.0, 0x06, 10, 0, 0, 20, 0)
        }
        "corpser" => tmpl(0x006C, 130, 40.0, 0x06, 20, 0, 20, 20, 0),
        "reaper" => tmpl(0x0065, 100, 35.0, 0x06, 15, 0, 30, 20, 0),

        // ----------------------------------------------------------------
        // Misc monsters
        // ----------------------------------------------------------------
        "wisp" | "crystalwisp" => tmpl(0x0058, 100, 35.0, 0x06, 10, 0, 0, 0, 20),
        "harpy" | "steppeharpy" | "arcticogrelord" => tmpl(0x0060, 150, 40.0, 0x06, 10, 0, 0, 0, 0),
        "gazer" | "eldergazer" => tmpl(0x0009, 100, 40.0, 0x06, 10, 0, 0, 0, 0),
        "seaserpent" | "crystalseaserpent" => tmpl(0x003E, 200, 50.0, 0x06, 20, 0, 10, 20, 0),
        "bullfrog" | "bulbousputrification" => tmpl(0x0051, 60, 20.0, 0x06, 5, 0, 0, 20, 0),
        "serpent" | "snake" | "silverserpent" | "giantsnake" => tmpl(0x0015, 35, 20.0, 0x06, 0, 0, 0, 10, 0),
        "coil" => tmpl(0x0015, 35, 20.0, 0x06, 0, 0, 0, 10, 0),

        // ----------------------------------------------------------------
        // Human criminals / fighters
        // ----------------------------------------------------------------
        "brigand" | "pirate" | "assassin" | "chaosguard" | "blackheart"
        | "factionhireling" => {
            tmpl(0x0190, 80, 40.0, 0x04, 10, 0, 0, 0, 0)
        }

        // ----------------------------------------------------------------
        // Named quest NPCs / townspeople – treat as innocent humans.
        // Any lowercase name that didn't match above falls through to the
        // default below; named NPCs are identified by short names without
        // spaces or digits.
        // ----------------------------------------------------------------
        _ => {
            // Heuristic: if it looks like a proper NPC class name with no
            // digits, assume it's a humanoid townsperson or named NPC.
            let has_digit = lower.chars().any(|c| c.is_ascii_digit());
            let is_humanoid = !has_digit && lower.len() >= 3;

            NpcTemplate {
                body_type: 0x0190,
                hue: 0,
                hit_points: if is_humanoid { 30_000 } else { 50 },
                defense_skill: if is_humanoid { 0.0 } else { 25.0 },
                notoriety: if is_humanoid { 0x01 } else { 0x06 },
                armor: ArmorStats {
                    physical_resist: 0,
                    fire_resist: 0,
                    cold_resist: 0,
                    poison_resist: 0,
                    energy_resist: 0,
                },
            }
        }
    })
}

/// Convenience constructor for `NpcTemplate`.
#[allow(clippy::too_many_arguments)]
fn tmpl(
    body_type: u16,
    hit_points: i32,
    defense_skill: f32,
    notoriety: u8,
    phys: i32,
    fire: i32,
    cold: i32,
    poison: i32,
    energy: i32,
) -> NpcTemplate {
    NpcTemplate {
        body_type,
        hue: 0,
        hit_points,
        defense_skill,
        notoriety,
        armor: ArmorStats {
            physical_resist: phys as i16,
            fire_resist: fire as i16,
            cold_resist: cold as i16,
            poison_resist: poison as i16,
            energy_resist: energy as i16,
        },
    }
}

// ---------------------------------------------------------------------------
// XML parser
// ---------------------------------------------------------------------------

/// Parse the first token (class name) from an `Objects2` value.
///
/// Format: `ClassName:MX=1:...:OBJ=ClassName2:MX=1:...`
/// We split on `:OBJ=` first to get per-type segments, then take the
/// substring before the first `:` in each segment.
fn parse_object_types(objects2: &str) -> Vec<String> {
    objects2
        .split(":OBJ=")
        .map(|seg| {
            seg.split(':').next().unwrap_or("").trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load NPCs from a ServUO-style `felucca.xml` file into `npc_manager`.
///
/// * `xml_path` — path to the XML file.
/// * `npc_manager` — target registry.
/// * `serial_start` — first serial to assign (use e.g. `0x0000_2000` to
///   leave room for hardcoded test NPCs in the `0x1000` range).
///
/// Returns the number of NPCs spawned, or an error string.
pub fn load_from_xml(
    xml_path: &str,
    npc_manager: &NpcManager,
    serial_start: u32,
) -> Result<usize, String> {
    let content = std::fs::read_to_string(xml_path)
        .map_err(|e| format!("Cannot read {xml_path}: {e}"))?;

    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();

    // Fields collected while inside a <Points> block.
    let mut in_points = false;
    let mut centre_x: i32 = 0;
    let mut centre_y: i32 = 0;
    let mut centre_z: i32 = 0;
    let mut is_running = false;
    let mut objects2 = String::new();
    let mut current_tag = String::new();

    let mut next_serial = serial_start;
    let mut spawned = 0usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                if tag == "Points" {
                    in_points = true;
                    centre_x = 0;
                    centre_y = 0;
                    centre_z = 0;
                    is_running = false;
                    objects2.clear();
                }
                current_tag = tag;
            }
            Ok(Event::Text(ref e)) if in_points => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                match current_tag.as_str() {
                    "CentreX" => centre_x = text.parse().unwrap_or(0),
                    "CentreY" => centre_y = text.parse().unwrap_or(0),
                    "CentreZ" => centre_z = text.parse().unwrap_or(0),
                    "IsRunning" => is_running = text.eq_ignore_ascii_case("true"),
                    "Objects2" => objects2 = text,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name();
                let tag = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
                if tag == "Points" && in_points {
                    in_points = false;

                    if is_running && !objects2.is_empty() {
                        let pos = Position {
                            x: centre_x.clamp(0, 0x7FFF) as u16,
                            y: centre_y.clamp(0, 0x7FFF) as u16,
                            z: centre_z.clamp(i8::MIN as i32, i8::MAX as i32) as i8,
                        };

                        for class_name in parse_object_types(&objects2) {
                            if let Some(tmpl) = template_for(&class_name) {
                                let npc = Npc {
                                    serial: next_serial,
                                    name: class_name.clone(),
                                    body_type: tmpl.body_type,
                                    hue: tmpl.hue,
                                    position: pos,
                                    map_id: 0, // Felucca
                                    direction: 0x04,
                                    hit_points: tmpl.hit_points,
                                    max_hit_points: tmpl.hit_points,
                                    armor: tmpl.armor,
                                    defense_skill: tmpl.defense_skill,
                                    is_alive: true,
                                    notoriety: tmpl.notoriety,
                                    flags: 0x00,
                                };
                                npc_manager.spawn(npc);
                                next_serial = next_serial.wrapping_add(1);
                                spawned += 1;
                            }
                        }
                    }

                    current_tag.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
        buf.clear();
    }

    Ok(spawned)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_object_types_single() {
        let result = parse_object_types(
            "Dragon:MX=1:SB=0:RT=0:TO=0:KL=0:RK=0:CA=1:DN=-1:DX=-1:SP=1:PR=-1",
        );
        assert_eq!(result, vec!["Dragon"]);
    }

    #[test]
    fn parse_object_types_multi() {
        let result = parse_object_types(
            "Rat:MX=1:SB=0:OBJ=Ratman:MX=1:SB=0:OBJ=Troll:MX=2:SB=0",
        );
        assert_eq!(result, vec!["Rat", "Ratman", "Troll"]);
    }

    #[test]
    fn template_dragon() {
        let t = template_for("dragon").unwrap();
        assert_eq!(t.body_type, 0x00C9);
        assert_eq!(t.notoriety, 0x06);
        assert_eq!(t.armor.fire_resist, 100);
    }

    #[test]
    fn template_banker_innocent() {
        let t = template_for("Banker").unwrap();
        assert_eq!(t.body_type, 0x0190);
        assert_eq!(t.notoriety, 0x01);
    }

    #[test]
    fn template_treasure_skipped() {
        assert!(template_for("TreasureLevel3").is_none());
        assert!(template_for("CrystalFieldTele").is_none());
    }

    #[test]
    fn load_from_xml_smoke() {
        let xml = r#"<Spawns>
  <Points>
    <CentreX>1500</CentreX>
    <CentreY>1640</CentreY>
    <CentreZ>10</CentreZ>
    <IsRunning>True</IsRunning>
    <Objects2>Dragon:MX=1:SB=0:RT=0</Objects2>
  </Points>
  <Points>
    <CentreX>200</CentreX>
    <CentreY>200</CentreY>
    <CentreZ>0</CentreZ>
    <IsRunning>False</IsRunning>
    <Objects2>Orc:MX=1:SB=0</Objects2>
  </Points>
</Spawns>"#;

        let dir = std::env::temp_dir();
        let path = dir.join("test_felucca.xml");
        std::fs::write(&path, xml).unwrap();

        let mgr = NpcManager::new();
        let count = load_from_xml(path.to_str().unwrap(), &mgr, 0x2000).unwrap();

        // Only the running point should spawn (1 dragon); orc point is stopped.
        assert_eq!(count, 1);
        let npc = mgr.get(0x2000).unwrap();
        assert_eq!(npc.body_type, 0x00C9);
        assert_eq!(npc.position.x, 1500);
    }
}
