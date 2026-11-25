//! Exposure classes of concrete: what the surroundings do to it, and the least strength class
//! that goes with it.
//!
//! The eighteen classes are EN 206's (table 1): X0, and by what attacks the concrete or its
//! reinforcement XC (carbonation), XD (chlorides other than of sea water), XS (chlorides of sea
//! water), XF (freezing and thawing) and XA (chemical attack). What each stands for is said
//! here in a few words of our own, not in the standard's.
//!
//! The least strength class of each is the indicative one of EN 1992-1-1, annex E (table
//! E.1N); that table has none for XF4, whose class is the one EN 206 gives (table F.1). They are
//! the recommended values: a country's annex may set others (the Dutch one does), none is
//! checked here, and nothing here is validated by an engineer.
//!
//! A material names its exposure classes ([`crate::Material::exposure`]): "XC4" and "XF1" on a
//! concrete for outside work. [`Material::below`] says when its strength class is under the
//! least one of a class it names.

use crate::class::{self, Family, StrengthClass};
use crate::{Lang, Material};

/// What attacks the concrete or the steel in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Attack {
    /// No risk of corrosion or attack.
    None,
    Carbonation,
    /// Chlorides other than of sea water.
    Chlorides,
    SeaWater,
    FreezeThaw,
    Chemical,
}

impl Attack {
    pub const ALL: [Attack; 6] = [Attack::None, Attack::Carbonation, Attack::Chlorides, Attack::SeaWater, Attack::FreezeThaw, Attack::Chemical];

    /// In Dutch, English, German and French.
    fn names(self) -> [&'static str; 4] {
        match self {
            Attack::None => ["geen aantasting", "no attack", "kein Angriff", "aucune attaque"],
            Attack::Carbonation => ["carbonatatie", "carbonation", "Karbonatisierung", "carbonatation"],
            Attack::Chlorides => ["chloriden, niet uit zeewater", "chlorides, not of sea water", "Chloride, nicht aus Meerwasser", "chlorures, hors eau de mer"],
            Attack::SeaWater => ["chloriden uit zeewater", "chlorides of sea water", "Chloride aus Meerwasser", "chlorures de l'eau de mer"],
            Attack::FreezeThaw => ["vorst en dooi", "freezing and thawing", "Frost und Tau", "gel et dégel"],
            Attack::Chemical => ["chemische aantasting", "chemical attack", "chemischer Angriff", "attaque chimique"],
        }
    }

    pub fn name(self, lang: Lang) -> &'static str {
        self.names()[at(lang)]
    }
}

fn at(lang: Lang) -> usize {
    match lang {
        Lang::Nl => 0,
        Lang::En => 1,
        Lang::De => 2,
        Lang::Fr => 3,
    }
}

/// An exposure class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exposure {
    /// "XC4".
    pub name: &'static str,
    pub attack: Attack,
    /// The surroundings it stands for, in Dutch, English, German and French.
    surroundings: [&'static str; 4],
    /// The least strength class that goes with it: the indicative one.
    pub least: &'static str,
}

impl Exposure {
    /// The surroundings it stands for.
    pub fn surroundings(&self, lang: Lang) -> &'static str {
        self.surroundings[at(lang)]
    }

    /// Its least strength class, as a class.
    pub fn least_class(&self) -> Option<&'static StrengthClass> {
        class::class(self.least)
    }

    /// The table its least strength class is from.
    pub fn source(&self) -> &'static str {
        if self.name == "XF4" {
            "EN 206, table F.1"
        } else {
            "EN 1992-1-1, table E.1N"
        }
    }

    /// "XC4 · carbonation: wet and dry in turn · C30/37".
    pub fn line(&self, lang: Lang) -> String {
        format!("{} · {}: {} · {}", self.name, self.attack.name(lang), self.surroundings(lang), self.least)
    }
}

const fn x(name: &'static str, attack: Attack, surroundings: [&'static str; 4], least: &'static str) -> Exposure {
    Exposure { name, attack, surroundings, least }
}

/// Every class, in the order of EN 206.
pub static CLASSES: [Exposure; 18] = [
    x("X0", Attack::None, ["zeer droog, of beton zonder wapening", "very dry, or concrete without reinforcement", "sehr trocken, oder Beton ohne Bewehrung", "très sec, ou béton non armé"], "C12/15"),
    x("XC1", Attack::Carbonation, ["droog of blijvend nat", "dry or always wet", "trocken oder ständig nass", "sec ou toujours humide"], "C20/25"),
    x("XC2", Attack::Carbonation, ["nat, zelden droog", "wet, seldom dry", "nass, selten trocken", "humide, rarement sec"], "C25/30"),
    x("XC3", Attack::Carbonation, ["matig vochtig", "moderately humid", "mäßig feucht", "humidité modérée"], "C30/37"),
    x("XC4", Attack::Carbonation, ["afwisselend nat en droog", "wet and dry in turn", "wechselnd nass und trocken", "tour à tour humide et sec"], "C30/37"),
    x("XD1", Attack::Chlorides, ["matig vochtig", "moderately humid", "mäßig feucht", "humidité modérée"], "C30/37"),
    x("XD2", Attack::Chlorides, ["nat, zelden droog", "wet, seldom dry", "nass, selten trocken", "humide, rarement sec"], "C30/37"),
    x("XD3", Attack::Chlorides, ["afwisselend nat en droog", "wet and dry in turn", "wechselnd nass und trocken", "tour à tour humide et sec"], "C35/45"),
    x("XS1", Attack::SeaWater, ["zout uit de lucht, niet in het water", "salt from the air, not in the water", "Salz aus der Luft, nicht im Wasser", "sel de l'air, hors de l'eau"], "C30/37"),
    x("XS2", Attack::SeaWater, ["blijvend onder water", "always under water", "ständig unter Wasser", "toujours sous l'eau"], "C35/45"),
    x("XS3", Attack::SeaWater, ["getijden, spatwater en nevel", "tides, splash and spray", "Gezeiten, Spritzwasser und Sprühnebel", "marées, éclaboussures et embruns"], "C35/45"),
    x("XF1", Attack::FreezeThaw, ["matig verzadigd, zonder dooizout", "moderately saturated, without de-icing salt", "mäßig gesättigt, ohne Taumittel", "saturation modérée, sans sel de déverglaçage"], "C30/37"),
    x("XF2", Attack::FreezeThaw, ["matig verzadigd, met dooizout", "moderately saturated, with de-icing salt", "mäßig gesättigt, mit Taumittel", "saturation modérée, avec sel de déverglaçage"], "C25/30"),
    x("XF3", Attack::FreezeThaw, ["sterk verzadigd, zonder dooizout", "highly saturated, without de-icing salt", "stark gesättigt, ohne Taumittel", "forte saturation, sans sel de déverglaçage"], "C30/37"),
    x("XF4", Attack::FreezeThaw, ["sterk verzadigd, met dooizout of zeewater", "highly saturated, with de-icing salt or sea water", "stark gesättigt, mit Taumittel oder Meerwasser", "forte saturation, avec sel de déverglaçage ou eau de mer"], "C30/37"),
    x("XA1", Attack::Chemical, ["zwak agressief", "slightly aggressive", "schwach angreifend", "faiblement agressif"], "C30/37"),
    x("XA2", Attack::Chemical, ["matig agressief", "moderately aggressive", "mäßig angreifend", "modérément agressif"], "C30/37"),
    x("XA3", Attack::Chemical, ["sterk agressief", "highly aggressive", "stark angreifend", "fortement agressif"], "C35/45"),
];

/// The class a name stands for, whatever its case and with spaces around it ("xc4").
pub fn exposure(name: &str) -> Option<&'static Exposure> {
    let name = name.trim();
    CLASSES.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// The classes of what attacks.
pub fn of(attack: Attack) -> impl Iterator<Item = &'static Exposure> {
    CLASSES.iter().filter(move |c| c.attack == attack)
}

/// Of `names`, the class that asks the strongest concrete; of two that ask the same, the first
/// in the order of the standard. None when none of them is a class.
pub fn governing<S: AsRef<str>>(names: &[S]) -> Option<&'static Exposure> {
    let strength = |c: &Exposure| c.least_class().and_then(StrengthClass::strength).unwrap_or(0.0);
    CLASSES.iter().filter(|c| names.iter().any(|n| exposure(n.as_ref()).is_some_and(|named| named.name == c.name))).fold(None, |best: Option<&Exposure>, c| match best {
        Some(best) if strength(best) >= strength(c) => Some(best),
        _ => Some(c),
    })
}

impl Material {
    /// The exposure classes it names, in the order of the standard; what it names that is no
    /// class is left out.
    pub fn exposures(&self) -> Vec<&'static Exposure> {
        CLASSES.iter().filter(|c| self.exposure.iter().any(|n| n.eq_ignore_ascii_case(c.name))).collect()
    }

    /// It names `class`, or names it no more. Kept in the order of the standard.
    pub fn expose(&mut self, class: &str, on: bool) {
        let Some(class) = exposure(class) else { return };
        self.exposure.retain(|n| !n.eq_ignore_ascii_case(class.name));
        if on {
            self.exposure.push(class.name.into());
        }
        let place = |n: &String| CLASSES.iter().position(|c| n.eq_ignore_ascii_case(c.name)).unwrap_or(CLASSES.len());
        self.exposure.sort_by_key(place);
    }

    /// The exposure class its concrete is too weak for: the one of those it names that asks the
    /// strongest concrete, when its own strength class is a concrete class under that. None when
    /// it is strong enough, names no class of exposure, or is no concrete of a strength class.
    pub fn below(&self) -> Option<&'static Exposure> {
        let own = self.class().filter(|c| c.family == Family::Concrete)?.strength()?;
        let governing = governing(&self.exposure)?;
        (own < governing.least_class()?.strength()?).then_some(governing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HatchKind, Names};

    #[test]
    fn the_exposure_classes_are_those_of_the_standard_each_with_a_least_strength_class() {
        assert_eq!(CLASSES.map(|c| c.name).join(" "), "X0 XC1 XC2 XC3 XC4 XD1 XD2 XD3 XS1 XS2 XS3 XF1 XF2 XF3 XF4 XA1 XA2 XA3");
        assert_eq!(Attack::ALL.map(|a| of(a).count()), [1, 4, 3, 3, 4, 3]);
        for c in &CLASSES {
            let least = c.least_class().unwrap_or_else(|| panic!("{}", c.name));
            assert_eq!(least.family, Family::Concrete, "{}", c.name);
            assert!(std::ptr::eq(exposure(&c.name.to_lowercase()).unwrap(), c));
            for lang in [Lang::Nl, Lang::En, Lang::De, Lang::Fr] {
                assert!(!c.surroundings(lang).is_empty() && !c.attack.name(lang).is_empty(), "{}", c.name);
            }
        }
        // the indicative classes of EN 1992-1-1, annex E
        let least = |name: &str| exposure(name).unwrap().least;
        assert_eq!(["X0", "XC1", "XC2", "XC3", "XC4"].map(least), ["C12/15", "C20/25", "C25/30", "C30/37", "C30/37"]);
        assert_eq!(["XD1", "XD2", "XD3", "XS1", "XS2", "XS3"].map(least), ["C30/37", "C30/37", "C35/45", "C30/37", "C35/45", "C35/45"]);
        assert_eq!(["XF1", "XF2", "XF3", "XF4", "XA1", "XA2", "XA3"].map(least), ["C30/37", "C25/30", "C30/37", "C30/37", "C30/37", "C30/37", "C35/45"]);
        // within what attacks, a harsher class asks no weaker concrete: but for XF2, where the
        // standard counts on entrained air
        for attack in [Attack::Carbonation, Attack::Chlorides, Attack::SeaWater, Attack::Chemical] {
            let fck: Vec<f64> = of(attack).map(|c| c.least_class().unwrap().strength().unwrap()).collect();
            assert!(fck.windows(2).all(|pair| pair[0] <= pair[1]), "{attack:?}");
        }
        assert_eq!((exposure(" xc4 ").map(|c| c.name), exposure("XC5"), exposure("")), (Some("XC4"), None, None));
        assert_eq!((exposure("XF4").unwrap().source(), exposure("XC1").unwrap().source()), ("EN 206, table F.1", "EN 1992-1-1, table E.1N"));
        assert_eq!(exposure("XC4").unwrap().line(Lang::En), "XC4 · carbonation: wet and dry in turn · C30/37");
        assert_eq!(exposure("XS3").unwrap().line(Lang::Nl), "XS3 · chloriden uit zeewater: getijden, spatwater en nevel · C35/45");
    }

    #[test]
    fn a_material_names_its_exposure_classes_and_is_told_when_it_is_too_weak() {
        let mut concrete = Material::new("concrete-outside", Names::same("concrete"), HatchKind::Concrete, [190, 190, 190]);
        assert!(concrete.exposures().is_empty() && concrete.below().is_none());
        // named in any order, kept in the order of the standard; no class is named twice
        concrete.expose("xf1", true);
        concrete.expose("XC4", true);
        concrete.expose("XC4", true);
        concrete.expose("XQ9", true);
        assert_eq!(concrete.exposure, ["XC4", "XF1"]);
        assert_eq!(concrete.exposures().iter().map(|c| c.name).collect::<Vec<_>>(), ["XC4", "XF1"]);
        // without a strength class nothing is said of its strength
        assert_eq!((governing(&concrete.exposure).map(|c| c.name), concrete.below()), (Some("XC4"), None));
        // too weak for what it stands in, strong enough, and stronger
        concrete.grade = Some("C20/25".into());
        assert_eq!(concrete.below().map(|c| (c.name, c.least)), Some(("XC4", "C30/37")));
        concrete.grade = Some("C30/37".into());
        assert!(concrete.below().is_none());
        // the class that asks the most decides
        concrete.expose("XD3", true);
        assert_eq!((concrete.exposure.join(" "), concrete.below().map(|c| c.name)), ("XC4 XD3 XF1".to_string(), Some("XD3")));
        concrete.grade = Some("C35/45".into());
        assert!(concrete.below().is_none());
        concrete.expose("XD3", false);
        concrete.expose("XF1", false);
        assert_eq!(concrete.exposure, ["XC4"]);
        // a grade that is no concrete class is not weighed
        concrete.grade = Some("S355".into());
        assert!(concrete.below().is_none());
        // names read from a file that are no class are kept and not counted
        concrete.exposure = vec!["xc2".into(), "W4".into()];
        assert_eq!((concrete.exposures().iter().map(|c| c.name).collect::<Vec<_>>(), governing(&concrete.exposure).map(|c| c.name)), (vec!["XC2"], Some("XC2")));
        // it goes to a file and comes back, and a material of before has none
        let node = crate::ifcx::material_node(&concrete);
        assert_eq!(node["attributes"]["bsi::ifc::prop::ExposureClass"].as_str(), Some("XC2"));
        let back: Material = serde_json::from_value(serde_json::to_value(&concrete).unwrap()).unwrap();
        assert_eq!(back, concrete);
        let plain = Material::new("brick", Names::same("brick"), HatchKind::Brick, [180, 90, 70]);
        let saved = serde_json::to_value(&plain).unwrap();
        assert!(saved.get("exposure").is_none() && crate::ifcx::material_node(&plain)["attributes"].get("bsi::ifc::prop::ExposureClass").is_none());
        assert!(serde_json::from_value::<Material>(saved).unwrap().exposure.is_empty());
    }
}
