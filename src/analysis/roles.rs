//! Per-role analysis configuration.
//!
//! Each Riot role (`TOP` / `JUNGLE` / `MIDDLE` / `BOTTOM` / `UTILITY`) declares
//! a [`RoleSpec`] that lists which shared skills (`cs_per_minute`,
//! `damage_per_minute`, `kills_assists`, `deaths`, `vision_score`) it
//! cares about, with role-specific thresholds + tactical notes that get
//! substituted into the skill markdown at prompt-composition time.
//!
//! The benchmarks below target the Platine/Émeraude range; they are
//! intentionally narrative (paragraph-style French) because they're spliced
//! verbatim into the prompt body sent to Gemini.

/// How much weight a role places on a given metric. Drives both
/// inclusion (`NotApplicable` skips the skill entirely) and framing in
/// the composed prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillImportance {
    Critical,
    High,
    Medium,
    Low,
    NotApplicable,
}

impl SkillImportance {
    /// French header label that prefaces the skill block.
    pub fn label_fr(self) -> Option<&'static str> {
        match self {
            SkillImportance::Critical => Some("**Métrique critique pour ce rôle**"),
            SkillImportance::High => Some("**Métrique importante**"),
            SkillImportance::Medium => Some("**À évaluer**"),
            SkillImportance::Low => Some("**À mentionner si pertinent**"),
            SkillImportance::NotApplicable => None,
        }
    }
}

/// Binding of a shared skill to a role-specific threshold + notes block.
pub struct SkillBinding {
    pub skill: &'static str,
    pub importance: SkillImportance,
    /// Substituted into the skill's `{benchmarks}` placeholder.
    pub benchmarks: &'static str,
    /// Substituted into the skill's `{role_notes}` placeholder.
    pub role_notes: &'static str,
}

pub struct RoleSpec {
    pub riot_role: &'static str,
    pub bindings: &'static [SkillBinding],
}

pub const ROLE_SPECS: &[RoleSpec] = &[TOP, JUNGLE, MIDDLE, BOTTOM, UTILITY];

pub const TOP: RoleSpec = RoleSpec {
    riot_role: "TOP",
    bindings: &[
        SkillBinding {
            skill: "cs_per_minute",
            importance: SkillImportance::Critical,
            benchmarks: "Repères Top en Platine/Émeraude :\n- Poor : < 6 CS/min sur la game\n- Average : 6-7,5 CS/min\n- Good : 7,5-9 CS/min (objectif solide)\n- Excellent : 9+ CS/min\n\nLane phase : 70-80 CS à 10 min, 130-150 CS à 20 min sont les repères pour bien jouer la lane.",
            role_notes: "En top, le CS est LA métrique de pression. Garder +10 CS sur l'adversaire à 10 min ouvre tout : roam jungle, prise de plaques, freeze gagnant.",
        },
        SkillBinding {
            skill: "damage_per_minute",
            importance: SkillImportance::Medium,
            benchmarks: "Repères Top en Platine/Émeraude :\n- Bruiser/fighter : 450-700 DPM attendu\n- Tank : 250-450 DPM (jugé surtout sur la KP et le frontline, pas sur le DPM)\n- Carry top (Jax, Fiora, Camille) : 600-900 DPM attendu",
            role_notes: "Un haut DPM en top sans pression de tourelle est souvent du carpet bombing inutile. Convertir = plaques, kills, roam.",
        },
        SkillBinding {
            skill: "kills_assists",
            importance: SkillImportance::Medium,
            benchmarks: "Repères Top en Platine/Émeraude :\n- KP attendue : 45-55% (top est la lane la plus isolée, KP plus basse que les autres rôles)\n- Une KP < 40% = jeu trop solo, lane non convertie en map pressure.\n- Une KP > 65% = sur-grouping qui coûte du CS de side.",
            role_notes: "La conversion d'avance solo lane vers la map se mesure en KP + tourelles + jungle invade. Pas juste en kills.",
        },
        SkillBinding {
            skill: "deaths",
            importance: SkillImportance::High,
            benchmarks: "Repères Top en Platine/Émeraude :\n- 0-3 morts : excellent contrôle\n- 4-6 morts : moyen, à surveiller\n- 7+ morts : trop. Souvent dû à un overextend en lane sans vision river.",
            role_notes: "Le top mourant donne directement Drake/Herald à l'ennemi car le jungler perd son back-up. Le ward 2:50-3:00 au river est obligatoire.",
        },
        SkillBinding {
            skill: "vision_score",
            importance: SkillImportance::Low,
            benchmarks: "Repères Top en Platine/Émeraude :\n- Objectif : 0,8-1,2 vision score/min\n- Ne pas critiquer un faible score si le contrôle de wave et le CS sont bons.",
            role_notes: "Pour le top, la vision est secondaire derrière le tempo de lane. Mais un control ward au river après push est gratuit.",
        },
    ],
};

pub const JUNGLE: RoleSpec = RoleSpec {
    riot_role: "JUNGLE",
    bindings: &[
        SkillBinding {
            skill: "cs_per_minute",
            importance: SkillImportance::Low,
            benchmarks: "Repères Jungle en Platine/Émeraude :\n- Le CS jungle vaut plus de gold/CS que la lane, mais le total est plus faible.\n- 4,5-5,5 CS/min (camps + minions) = correct.\n- Le tempo de clear (≤2:55 full clear) compte plus que le total brut.",
            role_notes: "Ne pas critiquer un faible CS pour un jungler qui a très bien gank. La métrique critique pour le jungler est la KP et le contrôle d'objectifs.",
        },
        SkillBinding {
            skill: "damage_per_minute",
            importance: SkillImportance::Medium,
            benchmarks: "Repères Jungle en Platine/Émeraude :\n- Carry jungle (Lee Sin, Graves, Kha'Zix) : 500-800 DPM\n- Tank/utilitaire (Amumu, Sejuani) : 250-400 DPM, jugé sur KP et présence aux objectifs",
            role_notes: "Le DPM jungle inclut les dégâts aux camps si on n'utilise pas total_damage_dealt_to_champions strictement. Toujours évaluer dans le contexte du type de champion.",
        },
        SkillBinding {
            skill: "kills_assists",
            importance: SkillImportance::Critical,
            benchmarks: "Repères Jungle en Platine/Émeraude :\n- KP attendue : 52-58% (le jungler doit avoir l'une des plus hautes KP de l'équipe).\n- Poor : < 45% (pathing trop passif, trop de full clear sans gank).\n- Good : 55-65%.\n- Au-delà de 70% = over-grouping et négligence du farm.",
            role_notes: "Le jungler vit et meurt par sa KP. Une KP basse + équipe qui perd la lane = pathing à reprendre depuis zéro.",
        },
        SkillBinding {
            skill: "deaths",
            importance: SkillImportance::High,
            benchmarks: "Repères Jungle en Platine/Émeraude :\n- 0-3 morts : excellent\n- 4-6 morts : moyen, surveiller les ganks ratés et les invades sans vision\n- 7+ : pathing brisé, ennemi contre-track avec succès",
            role_notes: "Chaque mort jungle = double info : ta position est confirmée + l'ennemi peut prendre l'objectif opposé. Une mort à 3 min = Drake quasi assuré.",
        },
        SkillBinding {
            skill: "vision_score",
            importance: SkillImportance::High,
            benchmarks: "Repères Jungle en Platine/Émeraude :\n- Objectif : 1,3-1,8 vision score/min\n- < 1,0 = invisible jungle pour son équipe.\n- Le contrôle de vision autour des objectifs (90s avant spawn) > le total brut.",
            role_notes: "La vision jungle prépare les objectifs. Sweeper le pit + control ward + ward à l'entrée river ennemie = setup standard à faire 60-90s avant chaque Drake/Herald/Baron.",
        },
    ],
};

pub const MIDDLE: RoleSpec = RoleSpec {
    riot_role: "MIDDLE",
    bindings: &[
        SkillBinding {
            skill: "cs_per_minute",
            importance: SkillImportance::High,
            benchmarks: "Repères Mid en Platine/Émeraude :\n- Poor : < 6,5 CS/min\n- Average : 6,5-8 CS/min\n- Good : 8-9 CS/min (objectif standard)\n- Excellent : 9+ CS/min\n\nLane phase : 80-90 CS à 10 min (mage de farm) ; 70-80 CS à 10 min (assassin/roamer).",
            role_notes: "Le mid est la lane la plus connectée à la map. Une avance de CS + un roam réussi = snowball maximal. Mais perdre du CS pour un roam raté = double pénalité.",
        },
        SkillBinding {
            skill: "damage_per_minute",
            importance: SkillImportance::Critical,
            benchmarks: "Repères Mid en Platine/Émeraude :\n- Control mage (Orianna, Viktor, Syndra) : 650-900 DPM\n- Burst mage (Syndra, Veigar, Lux) : 600-800 DPM\n- Assassin (Zed, Akali, Talon) : 450-650 DPM\n- Sous 450 DPM en tant que mid damage-oriented = trop de morts ou trop passif.",
            role_notes: "Pour le mid, le DPM EST la métrique de carry. Couplé au team_damage_percentage : un mid devrait être 1er ou 2e en dégâts d'équipe (25-32%).",
        },
        SkillBinding {
            skill: "kills_assists",
            importance: SkillImportance::High,
            benchmarks: "Repères Mid en Platine/Émeraude :\n- KP attendue : 50-58%\n- Poor : < 45% (mid trop passif, pas de roam)\n- Good : 55-65%\n- > 70% = over-grouping qui coûte des vagues mid",
            role_notes: "La KP mid se construit avec les roams. Un mid avec 60% KP et beaucoup d'assists = roamer efficace. Avec 60% KP et beaucoup de kills = carry qui finit les fights.",
        },
        SkillBinding {
            skill: "deaths",
            importance: SkillImportance::High,
            benchmarks: "Repères Mid en Platine/Émeraude :\n- 0-3 morts : excellent\n- 4-6 : moyen\n- 7+ : trop, souvent dû à des all-in mal calculés ou des roams sans vision",
            role_notes: "Pour un assassin, mourir après un pick raté = double pénalité (pas de kill + shutdown donné). Pour un mage, mourir en lane à l'ennemi assassin = lane perdue pour le reste de la game.",
        },
        SkillBinding {
            skill: "vision_score",
            importance: SkillImportance::Medium,
            benchmarks: "Repères Mid en Platine/Émeraude :\n- Objectif : 1,0-1,5 vision score/min\n- < 0,8 = pas de wards sur les approches Drake/Herald, ce qui coûte des objectifs.",
            role_notes: "Le mid est responsable de la vision river offensive (côté Drake et Herald). Un control ward au river après un push de vague est obligatoire.",
        },
    ],
};

pub const BOTTOM: RoleSpec = RoleSpec {
    riot_role: "BOTTOM",
    bindings: &[
        SkillBinding {
            skill: "cs_per_minute",
            importance: SkillImportance::Critical,
            benchmarks: "Repères ADC en Platine/Émeraude :\n- Poor : < 6,5 CS/min\n- Average : 6,5-8 CS/min\n- Good : 8-9 CS/min (objectif standard)\n- Excellent : 9-10+ CS/min\n\nLane phase : 80-90 CS à 10 min = correct ; 90+ = bon.",
            role_notes: "Pour l'ADC, le CS = items = scaling = late game. Chaque mort coûte ~25 CS minimum = un composant d'item. Un ADC qui reste en bot après chute T1 sans raison perd du CS comparé à un ADC qui swap mid après T1.",
        },
        SkillBinding {
            skill: "damage_per_minute",
            importance: SkillImportance::Critical,
            benchmarks: "Repères ADC en Platine/Émeraude :\n- Poor : < 600 DPM\n- Average : 600-800 DPM\n- Good : 800-1100 DPM\n- Excellent : 1100+ DPM\n\nL'ADC devrait être 1er en team_damage_percentage (25-35% des dégâts d'équipe).",
            role_notes: "Le DPM ADC est la métrique de succès numéro 2 après la survie. Un ADC qui inflige peu de dégâts = ADC qui est mort tôt en teamfight ou qui est resté trop à l'arrière.",
        },
        SkillBinding {
            skill: "kills_assists",
            importance: SkillImportance::High,
            benchmarks: "Repères ADC en Platine/Émeraude :\n- KP attendue : 50-60%\n- Poor : < 45%\n- Good : 55-65%",
            role_notes: "L'ADC se mesure plus en dégâts qu'en KP, mais une KP < 45% indique un ADC absent des fights — soit mort trop tôt, soit en train de farmer pendant les objectifs.",
        },
        SkillBinding {
            skill: "deaths",
            importance: SkillImportance::Critical,
            benchmarks: "Repères ADC en Platine/Émeraude :\n- 0-3 morts : excellent positionnement\n- 4-6 : moyen, à surveiller (les pros font 2-3)\n- 7+ : positionnement défaillant, ADC qui ne peut pas tenir le late game",
            role_notes: "Règle #1 ADC : 0 dégâts mort > beaucoup de dégâts en mourant. Chaque mort = l'équipe perd 100% de ses dégâts soutenus pendant le respawn.",
        },
        SkillBinding {
            skill: "vision_score",
            importance: SkillImportance::Low,
            benchmarks: "Repères ADC en Platine/Émeraude :\n- Objectif : 0,8-1,2 vision score/min\n- Ne pas critiquer un faible score, c'est le rôle du support.",
            role_notes: "L'ADC achète un control ward par recall en bot lane phase. Après ça, la vision est portée par le support et le jungler.",
        },
    ],
};

pub const UTILITY: RoleSpec = RoleSpec {
    riot_role: "UTILITY",
    bindings: &[
        SkillBinding {
            skill: "cs_per_minute",
            importance: SkillImportance::NotApplicable,
            benchmarks: "",
            role_notes: "",
        },
        SkillBinding {
            skill: "damage_per_minute",
            importance: SkillImportance::Low,
            benchmarks: "Repères Support en Platine/Émeraude :\n- Mage support (Brand, Zyra, Xerath) : 400-650 DPM attendu\n- Enchanteur (Lulu, Nami, Soraka) : 150-350 DPM (juge sur les heals/buffs/KP, pas sur le DPM)\n- Tank/engage (Leona, Nautilus) : 200-400 DPM",
            role_notes: "Pour la plupart des supports, le DPM est une métrique secondaire. Ne le mentionne que pour les mages supports ou si le support a porté toute l'équipe.",
        },
        SkillBinding {
            skill: "kills_assists",
            importance: SkillImportance::Critical,
            benchmarks: "Repères Support en Platine/Émeraude :\n- KP attendue : 60-75% (la plus haute de l'équipe).\n- Engage (Leona, Nautilus) : 65-80% attendu.\n- Enchanteur (Nami, Lulu) : 55-70%.\n- Roamer (Bard, Pyke) : 70%+ attendu.\n- < 50% KP = autopilot en lane pendant que l'équipe fight sans toi.",
            role_notes: "La KP est LA métrique du support. Les assists comptent autant que les kills — un support qui peel et permet à son ADC de carry est un excellent support.",
        },
        SkillBinding {
            skill: "deaths",
            importance: SkillImportance::High,
            benchmarks: "Repères Support en Platine/Émeraude :\n- 0-4 morts : excellent\n- 5-7 : moyen (le support engage et meurt parfois pour sauver l'ADC — c'est attendu)\n- 8+ : trop, souvent dû à des engages 2v5 ou à chasing un kill quand l'ennemi a un escape",
            role_notes: "Une mort \"pour sauver l'ADC\" est attendue. Une mort \"pour rien\" (chase, engage 2v5) est le piège classique du support Platine.",
        },
        SkillBinding {
            skill: "vision_score",
            importance: SkillImportance::Critical,
            benchmarks: "Repères Support en Platine/Émeraude :\n- Poor : < 2,0 vision score/min\n- Average : 2,0-2,5\n- Good : 2,5-3,0 (Platine), 3,0+ (Émeraude)\n- Excellent : 3,5+\n\nObjectif concret : 10-15 wards posées par partie, 2 pièces de vision à chaque objectif majeur 90s avant spawn.",
            role_notes: "Le score brut est trompeur — c'est l'emplacement qui gagne les games. Un control ward acheté à CHAQUE recall (75g) est obligatoire. Switch vers Oracle Lens (sweeper) après la complétion de la quête de rôle.",
        },
    ],
};

/// Look up a [`RoleSpec`] by Riot role string (`"TOP"`, `"JUNGLE"`, ...).
pub fn spec_for(riot_role: &str) -> Option<&'static RoleSpec> {
    ROLE_SPECS.iter().find(|spec| spec.riot_role == riot_role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_has_all_five_skills() {
        for spec in ROLE_SPECS {
            let skills: Vec<&str> = spec.bindings.iter().map(|b| b.skill).collect();
            for required in &[
                "cs_per_minute",
                "damage_per_minute",
                "kills_assists",
                "deaths",
                "vision_score",
            ] {
                assert!(
                    skills.contains(required),
                    "role {} is missing binding for {required}",
                    spec.riot_role
                );
            }
        }
    }

    #[test]
    fn spec_lookup_resolves_known_roles() {
        for role in ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"] {
            assert!(spec_for(role).is_some(), "missing spec for {role}");
        }
    }

    #[test]
    fn spec_lookup_returns_none_for_unknown_role() {
        assert!(spec_for("INVALID").is_none());
    }

    #[test]
    fn support_does_not_grade_cs_per_minute() {
        let support = spec_for("UTILITY").unwrap();
        let cs = support
            .bindings
            .iter()
            .find(|b| b.skill == "cs_per_minute")
            .expect("cs_per_minute binding");
        assert_eq!(cs.importance, SkillImportance::NotApplicable);
    }

    #[test]
    fn adc_cares_critically_about_cs_and_damage() {
        let adc = spec_for("BOTTOM").unwrap();
        let cs = adc
            .bindings
            .iter()
            .find(|b| b.skill == "cs_per_minute")
            .unwrap();
        let dpm = adc
            .bindings
            .iter()
            .find(|b| b.skill == "damage_per_minute")
            .unwrap();
        assert_eq!(cs.importance, SkillImportance::Critical);
        assert_eq!(dpm.importance, SkillImportance::Critical);
    }

    #[test]
    fn support_cares_critically_about_vision_and_kp() {
        let support = spec_for("UTILITY").unwrap();
        let vision = support
            .bindings
            .iter()
            .find(|b| b.skill == "vision_score")
            .unwrap();
        let ka = support
            .bindings
            .iter()
            .find(|b| b.skill == "kills_assists")
            .unwrap();
        assert_eq!(vision.importance, SkillImportance::Critical);
        assert_eq!(ka.importance, SkillImportance::Critical);
    }
}
