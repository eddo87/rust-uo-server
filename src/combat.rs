use rand::Rng;

/// The type of elemental (or physical) damage dealt by an attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageType {
    Physical,
    Fire,
    Cold,
    Poison,
    Energy,
}

/// The weapon skill category used by a weapon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    Fists,
    Swords,
    Macing,
    Fencing,
    Archery,
    Throwing,
    Wrestling,
}

/// Stats describing a weapon's combat characteristics.
#[derive(Debug, Clone)]
pub struct WeaponStats {
    pub min_damage: i16,
    pub max_damage: i16,
    /// Base weapon speed value (higher = faster in the swing delay formula).
    pub speed: i16,
    pub weapon_type: WeaponType,
    /// 1 for melee weapons, >1 for ranged weapons (measured in tiles).
    pub range: i16,
}

/// Armor resistances across all five damage types. Values are percentages (0-70 typical range).
#[derive(Debug, Clone)]
pub struct ArmorStats {
    pub physical_resist: i16,
    pub fire_resist: i16,
    pub cold_resist: i16,
    pub poison_resist: i16,
    pub energy_resist: i16,
}

/// The outcome of a single combat round.
#[derive(Debug, Clone)]
pub struct CombatResult {
    pub damage_dealt: i16,
    pub damage_type: DamageType,
    pub was_hit: bool,
    pub was_critical: bool,
}

/// Calculate hit chance using the classic UO formula.
///
/// Formula: 50% + ((attacker_skill - defender_skill) * 0.5), clamped to [2%, 98%].
///
/// Both `attacker_skill` and `defender_skill` are expected in the 0.0-120.0 range
/// (matching UO's 0-120 skill point scale).
///
/// Returns a value between 0.02 and 0.98 (i.e. 2%-98% as a fraction).
pub fn calculate_hit_chance(attacker_skill: f32, defender_skill: f32) -> f32 {
    let chance = 0.50 + (attacker_skill - defender_skill) * 0.005;
    chance.clamp(0.02, 0.98)
}

/// Calculate base damage from weapon min/max and a strength bonus.
///
/// Rolls a random value in [min, max] then applies `strength_bonus` as a percentage
/// increase (e.g. strength_bonus=25 adds 25% to the base roll).
///
/// The minimum return value is 1 (attacks always deal at least 1 damage).
pub fn calculate_damage(min: i16, max: i16, strength_bonus: i16) -> i16 {
    let base = if min >= max {
        min
    } else {
        rand::thread_rng().gen_range(min..=max)
    };
    let bonus_multiplier = 1.0 + (strength_bonus as f32 / 100.0);
    let result = (base as f32 * bonus_multiplier) as i16;
    result.max(1)
}

/// Deterministic version of calculate_damage for testing.
/// Takes a pre-rolled `base` value instead of generating one randomly.
pub fn calculate_damage_with_base(base: i16, strength_bonus: i16) -> i16 {
    let bonus_multiplier = 1.0 + (strength_bonus as f32 / 100.0);
    let result = (base as f32 * bonus_multiplier) as i16;
    result.max(1)
}

/// Reduce damage by the given resistance percentage.
///
/// `resist` is a percentage (e.g. 30 means 30% damage reduction).
/// The minimum return value is 0 (damage cannot go negative).
pub fn apply_armor_reduction(damage: i16, resist: i16) -> i16 {
    let reduction = damage as f32 * (resist as f32 / 100.0);
    let result = damage - reduction as i16;
    result.max(0)
}

/// Calculate the delay between weapon swings in seconds.
///
/// Uses a simplified version of the classic UO swing delay formula:
///   delay = 5.0 - ((speed + stamina_bonus) as f32 * 0.025)
///
/// where stamina_bonus = (stamina + dexterity) / 2.
///
/// The result is clamped to [1.25, 5.0] seconds.
pub fn calculate_swing_delay(speed: i16, stamina: i16, dexterity: i16) -> f32 {
    let stamina_bonus = (stamina + dexterity) / 2;
    let delay = 5.0 - ((speed + stamina_bonus) as f32 * 0.025);
    delay.clamp(1.25, 5.0)
}

/// Resolve a full combat round: determine hit/miss, roll damage, apply armor.
///
/// This is the main entry point for combat resolution. It rolls for hit chance,
/// calculates damage on hit (with strength bonus and armor reduction against
/// physical resistance), and returns a `CombatResult`.
///
/// Critical hits occur on roughly 5% of successful hits and deal 1.5x damage.
pub fn resolve_combat_round(
    attacker_skill: f32,
    attacker_weapon: &WeaponStats,
    attacker_str: i16,
    defender_skill: f32,
    defender_armor: &ArmorStats,
) -> CombatResult {
    let mut rng = rand::thread_rng();

    let hit_chance = calculate_hit_chance(attacker_skill, defender_skill);
    let hit_roll: f32 = rng.gen();
    let was_hit = hit_roll < hit_chance;

    if !was_hit {
        return CombatResult {
            damage_dealt: 0,
            damage_type: DamageType::Physical,
            was_hit: false,
            was_critical: false,
        };
    }

    // Strength bonus: each point of STR above 0 adds ~0.5% damage
    let strength_bonus = attacker_str / 2;
    let base_damage = calculate_damage(
        attacker_weapon.min_damage,
        attacker_weapon.max_damage,
        strength_bonus,
    );

    // Critical hit: 5% chance on a successful hit, deals 1.5x damage
    let crit_roll: f32 = rng.gen();
    let was_critical = crit_roll < 0.05;
    let damage_after_crit = if was_critical {
        (base_damage as f32 * 1.5) as i16
    } else {
        base_damage
    };

    let final_damage = apply_armor_reduction(damage_after_crit, defender_armor.physical_resist);

    CombatResult {
        damage_dealt: final_damage.max(1), // Successful hits always deal at least 1
        damage_type: DamageType::Physical,
        was_hit: true,
        was_critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // calculate_hit_chance tests
    // -----------------------------------------------------------------------

    #[test]
    fn hit_chance_equal_skills() {
        let chance = calculate_hit_chance(50.0, 50.0);
        assert!(
            (chance - 0.50).abs() < 1e-6,
            "Equal skills should give 50% hit chance, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_attacker_advantage() {
        // 100 vs 50: 50% + (50 * 0.5)% = 50% + 25% = 75%
        let chance = calculate_hit_chance(100.0, 50.0);
        assert!(
            (chance - 0.75).abs() < 1e-6,
            "100 vs 50 should give 75% hit chance, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_defender_advantage() {
        // 50 vs 100: 50% + (-50 * 0.5)% = 50% - 25% = 25%
        let chance = calculate_hit_chance(50.0, 100.0);
        assert!(
            (chance - 0.25).abs() < 1e-6,
            "50 vs 100 should give 25% hit chance, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_clamps_to_minimum() {
        // 0 vs 120: 50% + (-120 * 0.5)% = 50% - 60% = -10% -> clamped to 2%
        let chance = calculate_hit_chance(0.0, 120.0);
        assert!(
            (chance - 0.02).abs() < 1e-6,
            "0 vs 120 should clamp to 2%, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_clamps_to_maximum() {
        // 120 vs 0: 50% + (120 * 0.5)% = 50% + 60% = 110% -> clamped to 98%
        let chance = calculate_hit_chance(120.0, 0.0);
        assert!(
            (chance - 0.98).abs() < 1e-6,
            "120 vs 0 should clamp to 98%, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_zero_skills() {
        let chance = calculate_hit_chance(0.0, 0.0);
        assert!(
            (chance - 0.50).abs() < 1e-6,
            "Both zero should give 50%, got {}",
            chance
        );
    }

    #[test]
    fn hit_chance_max_skills() {
        let chance = calculate_hit_chance(120.0, 120.0);
        assert!(
            (chance - 0.50).abs() < 1e-6,
            "Both max should give 50%, got {}",
            chance
        );
    }

    // -----------------------------------------------------------------------
    // calculate_damage_with_base tests (deterministic)
    // -----------------------------------------------------------------------

    #[test]
    fn damage_no_strength_bonus() {
        let dmg = calculate_damage_with_base(10, 0);
        assert_eq!(dmg, 10, "No bonus should return base damage");
    }

    #[test]
    fn damage_with_50_percent_bonus() {
        // base 10, +50% = 15
        let dmg = calculate_damage_with_base(10, 50);
        assert_eq!(dmg, 15, "10 base + 50% bonus should be 15, got {}", dmg);
    }

    #[test]
    fn damage_with_100_percent_bonus() {
        // base 10, +100% = 20
        let dmg = calculate_damage_with_base(10, 100);
        assert_eq!(dmg, 20, "10 base + 100% bonus should be 20, got {}", dmg);
    }

    #[test]
    fn damage_minimum_is_one() {
        // base 0, 0% bonus -> should clamp to 1
        let dmg = calculate_damage_with_base(0, 0);
        assert_eq!(dmg, 1, "Minimum damage should be 1, got {}", dmg);
    }

    #[test]
    fn damage_with_large_base() {
        let dmg = calculate_damage_with_base(100, 25);
        assert_eq!(dmg, 125, "100 base + 25% bonus should be 125, got {}", dmg);
    }

    #[test]
    fn damage_small_base_with_bonus() {
        // base 1, +50% = 1.5 -> truncated to 1
        let dmg = calculate_damage_with_base(1, 50);
        assert_eq!(dmg, 1, "1 base + 50% should truncate to 1, got {}", dmg);
    }

    #[test]
    fn damage_base_2_with_50_bonus() {
        // base 2, +50% = 3
        let dmg = calculate_damage_with_base(2, 50);
        assert_eq!(dmg, 3, "2 base + 50% should be 3, got {}", dmg);
    }

    // -----------------------------------------------------------------------
    // calculate_damage tests (random, bounds-checking)
    // -----------------------------------------------------------------------

    #[test]
    fn damage_random_within_bounds() {
        // Run multiple times to verify the result is always within a plausible range
        for _ in 0..100 {
            let dmg = calculate_damage(5, 15, 0);
            assert!(
                dmg >= 5 && dmg <= 15,
                "Damage {} out of [5, 15] range",
                dmg
            );
        }
    }

    #[test]
    fn damage_random_min_equals_max() {
        let dmg = calculate_damage(10, 10, 0);
        assert_eq!(dmg, 10, "min==max should always return that value");
    }

    #[test]
    fn damage_random_min_greater_than_max() {
        // When min > max, we use min as the base (degenerate case)
        let dmg = calculate_damage(15, 5, 0);
        assert_eq!(dmg, 15, "min > max should use min as base, got {}", dmg);
    }

    // -----------------------------------------------------------------------
    // apply_armor_reduction tests
    // -----------------------------------------------------------------------

    #[test]
    fn armor_zero_resist() {
        let result = apply_armor_reduction(100, 0);
        assert_eq!(result, 100, "0% resist should not reduce damage");
    }

    #[test]
    fn armor_50_resist() {
        let result = apply_armor_reduction(100, 50);
        assert_eq!(result, 50, "50% resist on 100 dmg should be 50, got {}", result);
    }

    #[test]
    fn armor_100_resist() {
        let result = apply_armor_reduction(100, 100);
        assert_eq!(result, 0, "100% resist should reduce to 0, got {}", result);
    }

    #[test]
    fn armor_small_damage_high_resist() {
        // 1 damage, 70% resist: 1 - 0.7 -> 1 - 0 = 1 (truncation)
        let result = apply_armor_reduction(1, 70);
        assert_eq!(result, 1, "Small damage with truncation, got {}", result);
    }

    #[test]
    fn armor_no_negative_damage() {
        // Edge case: resist > 100 should not produce negative damage
        let result = apply_armor_reduction(10, 150);
        assert_eq!(result, 0, "Excessive resist should clamp to 0, got {}", result);
    }

    #[test]
    fn armor_zero_damage() {
        let result = apply_armor_reduction(0, 50);
        assert_eq!(result, 0, "0 damage should stay 0, got {}", result);
    }

    #[test]
    fn armor_typical_values() {
        // 30 damage, 25% resist -> 30 - 7 = 23 (7.5 truncated to 7)
        let result = apply_armor_reduction(30, 25);
        assert_eq!(result, 23, "30 dmg, 25% resist should be 23, got {}", result);
    }

    // -----------------------------------------------------------------------
    // calculate_swing_delay tests
    // -----------------------------------------------------------------------

    #[test]
    fn swing_delay_low_stats() {
        // speed=10, stamina=10, dex=10 -> bonus=10, delay = 5.0 - (20 * 0.025) = 4.5
        let delay = calculate_swing_delay(10, 10, 10);
        assert!(
            (delay - 4.5).abs() < 1e-6,
            "Expected 4.5s swing delay, got {}",
            delay
        );
    }

    #[test]
    fn swing_delay_high_stats() {
        // speed=50, stamina=100, dex=100 -> bonus=100, delay = 5.0 - (150 * 0.025) = 1.25
        let delay = calculate_swing_delay(50, 100, 100);
        assert!(
            (delay - 1.25).abs() < 1e-6,
            "Expected 1.25s swing delay, got {}",
            delay
        );
    }

    #[test]
    fn swing_delay_clamps_to_minimum() {
        // Very high speed + stats should clamp to 1.25
        let delay = calculate_swing_delay(100, 150, 150);
        assert!(
            (delay - 1.25).abs() < 1e-6,
            "Should clamp to 1.25s minimum, got {}",
            delay
        );
    }

    #[test]
    fn swing_delay_clamps_to_maximum() {
        // All zeros -> 5.0 - 0 = 5.0
        let delay = calculate_swing_delay(0, 0, 0);
        assert!(
            (delay - 5.0).abs() < 1e-6,
            "Should be 5.0s maximum, got {}",
            delay
        );
    }

    #[test]
    fn swing_delay_moderate_stats() {
        // speed=30, stamina=50, dex=50 -> bonus=50, delay = 5.0 - (80 * 0.025) = 3.0
        let delay = calculate_swing_delay(30, 50, 50);
        assert!(
            (delay - 3.0).abs() < 1e-6,
            "Expected 3.0s swing delay, got {}",
            delay
        );
    }

    #[test]
    fn swing_delay_odd_stamina_dex_truncation() {
        // speed=20, stamina=25, dex=30 -> bonus=(25+30)/2=27, delay = 5.0 - (47*0.025) = 3.825
        let delay = calculate_swing_delay(20, 25, 30);
        assert!(
            (delay - 3.825).abs() < 1e-3,
            "Expected ~3.825s swing delay, got {}",
            delay
        );
    }

    // -----------------------------------------------------------------------
    // resolve_combat_round tests
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_combat_round_miss_returns_zero_damage() {
        let weapon = WeaponStats {
            min_damage: 10,
            max_damage: 20,
            speed: 30,
            weapon_type: WeaponType::Swords,
            range: 1,
        };
        let armor = ArmorStats {
            physical_resist: 20,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
        };

        // With 0 attacker skill vs 120 defender skill, hit chance = 2%.
        // Run many rounds; most will miss.
        let mut miss_count = 0;
        for _ in 0..100 {
            let result = resolve_combat_round(0.0, &weapon, 50, 120.0, &armor);
            if !result.was_hit {
                assert_eq!(result.damage_dealt, 0, "Misses should deal 0 damage");
                miss_count += 1;
            }
        }
        // With 2% hit chance, expect the vast majority to be misses
        assert!(
            miss_count > 80,
            "Expected most rounds to miss with 2% chance, got {} misses out of 100",
            miss_count
        );
    }

    #[test]
    fn resolve_combat_round_hit_deals_positive_damage() {
        let weapon = WeaponStats {
            min_damage: 10,
            max_damage: 20,
            speed: 30,
            weapon_type: WeaponType::Macing,
            range: 1,
        };
        let armor = ArmorStats {
            physical_resist: 10,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
        };

        // With 120 attacker vs 0 defender, hit chance = 98%.
        // Run many rounds; hits should deal positive damage.
        let mut hit_count = 0;
        for _ in 0..100 {
            let result = resolve_combat_round(120.0, &weapon, 100, 0.0, &armor);
            if result.was_hit {
                assert!(
                    result.damage_dealt >= 1,
                    "Hits should deal at least 1 damage, got {}",
                    result.damage_dealt
                );
                hit_count += 1;
            }
        }
        assert!(
            hit_count > 80,
            "Expected most rounds to hit with 98% chance, got {} hits out of 100",
            hit_count
        );
    }

    #[test]
    fn resolve_combat_round_result_type_is_physical() {
        let weapon = WeaponStats {
            min_damage: 5,
            max_damage: 10,
            speed: 25,
            weapon_type: WeaponType::Fencing,
            range: 1,
        };
        let armor = ArmorStats {
            physical_resist: 0,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
        };

        let result = resolve_combat_round(100.0, &weapon, 50, 0.0, &armor);
        assert_eq!(
            result.damage_type,
            DamageType::Physical,
            "Default damage type should be Physical"
        );
    }

    #[test]
    fn resolve_combat_round_armor_reduces_damage() {
        let weapon = WeaponStats {
            min_damage: 20,
            max_damage: 20, // fixed damage for predictability
            speed: 30,
            weapon_type: WeaponType::Swords,
            range: 1,
        };
        let no_armor = ArmorStats {
            physical_resist: 0,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
        };
        let heavy_armor = ArmorStats {
            physical_resist: 50,
            fire_resist: 0,
            cold_resist: 0,
            poison_resist: 0,
            energy_resist: 0,
        };

        // Collect hit damages with no armor vs heavy armor over many rounds
        let mut no_armor_total = 0i64;
        let mut heavy_armor_total = 0i64;
        let mut no_armor_hits = 0i64;
        let mut heavy_armor_hits = 0i64;

        for _ in 0..500 {
            let r1 = resolve_combat_round(120.0, &weapon, 0, 0.0, &no_armor);
            if r1.was_hit {
                no_armor_total += r1.damage_dealt as i64;
                no_armor_hits += 1;
            }
            let r2 = resolve_combat_round(120.0, &weapon, 0, 0.0, &heavy_armor);
            if r2.was_hit {
                heavy_armor_total += r2.damage_dealt as i64;
                heavy_armor_hits += 1;
            }
        }

        if no_armor_hits > 0 && heavy_armor_hits > 0 {
            let avg_no = no_armor_total as f64 / no_armor_hits as f64;
            let avg_heavy = heavy_armor_total as f64 / heavy_armor_hits as f64;
            assert!(
                avg_heavy < avg_no,
                "Heavy armor avg ({}) should be less than no armor avg ({})",
                avg_heavy,
                avg_no
            );
        }
    }

    // -----------------------------------------------------------------------
    // Struct / enum sanity tests
    // -----------------------------------------------------------------------

    #[test]
    fn weapon_stats_construction() {
        let bow = WeaponStats {
            min_damage: 15,
            max_damage: 25,
            speed: 35,
            weapon_type: WeaponType::Archery,
            range: 10,
        };
        assert_eq!(bow.weapon_type, WeaponType::Archery);
        assert!(bow.range > 1, "Archery should be ranged");
    }

    #[test]
    fn armor_stats_construction() {
        let plate = ArmorStats {
            physical_resist: 50,
            fire_resist: 5,
            cold_resist: 5,
            poison_resist: 5,
            energy_resist: 5,
        };
        assert_eq!(plate.physical_resist, 50);
    }

    #[test]
    fn damage_type_variants() {
        // Ensure all variants are distinct
        let types = [
            DamageType::Physical,
            DamageType::Fire,
            DamageType::Cold,
            DamageType::Poison,
            DamageType::Energy,
        ];
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j], "DamageType variants should be distinct");
            }
        }
    }

    #[test]
    fn weapon_type_variants() {
        let types = [
            WeaponType::Fists,
            WeaponType::Swords,
            WeaponType::Macing,
            WeaponType::Fencing,
            WeaponType::Archery,
            WeaponType::Throwing,
            WeaponType::Wrestling,
        ];
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j], "WeaponType variants should be distinct");
            }
        }
    }
}
