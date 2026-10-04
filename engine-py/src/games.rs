//! [`Game`] for Tank Arena and Racing, and the game list.

use crate::Game;
#[cfg(doc)]
use engine::generic::Flat;
use engine::generic::{Policy, Rules};
use engine::TankRules;
use game_catalog::{errors_json, BuildError, Levels, Valid};
use racing::RacingRules;

/// A validated list of builds: the match config and, per agent, the build's scripted
/// behavior id (`None` for a champion build, which only a learning agent can play).
#[derive(Debug)]
pub struct Lineup<R: Rules> {
    /// The config every match of this lineup starts from.
    pub config: R::Config,
    /// Per agent: the build's scripted behavior, or `None` for a champion.
    pub behaviors: Vec<Option<&'static str>>,
}

impl<R: Rules> Clone for Lineup<R>
where
    R::Config: Clone,
{
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            behaviors: self.behaviors.clone(),
        }
    }
}

/// One entry of [`GAMES`].
#[derive(Clone, Copy, Debug)]
pub struct GameInfo {
    /// Game id (`tank`, `racing`).
    pub id: &'static str,
    /// The rules version builds and replays refer to.
    pub rules_version: u64,
    /// [`Flat::OBS_LEN`].
    pub obs_len: usize,
    /// [`Flat::ACTION_LEN`].
    pub action_len: usize,
    /// [`Game::MIN_AGENTS`].
    pub min_agents: usize,
    /// [`Game::MAX_AGENTS`].
    pub max_agents: usize,
    /// The game's catalog JSON.
    pub catalog_json: &'static str,
    /// The game's default build JSON.
    pub default_build_json: &'static str,
    /// `validateBuild` for this game, as JSON ([`game_catalog::validation_json`]).
    pub validate_json: fn(&str) -> String,
}

const fn info<R: Game>(
    rules_version: u64,
    catalog_json: &'static str,
    default_build_json: &'static str,
    validate_json: fn(&str) -> String,
) -> GameInfo
where
    R::Config: Clone,
{
    GameInfo {
        id: R::GAME,
        rules_version,
        obs_len: R::OBS_LEN,
        action_len: R::ACTION_LEN,
        min_agents: R::MIN_AGENTS,
        max_agents: R::MAX_AGENTS,
        catalog_json,
        default_build_json,
        validate_json,
    }
}

/// Every game the bindings expose, in `games()` order (the same as engine-wasm's).
pub const GAMES: [GameInfo; 2] = [
    info::<TankRules>(
        tank::catalog::RULES_VERSION,
        tank::catalog::CATALOG_JSON,
        tank::catalog::DEFAULT_BUILD_JSON,
        |json| game_catalog::validation_json(&tank::catalog::validate_build(json)),
    ),
    info::<RacingRules>(
        racing::catalog::RULES_VERSION,
        racing::catalog::CATALOG_JSON,
        racing::catalog::DEFAULT_BUILD_JSON,
        |json| game_catalog::validation_json(&racing::catalog::validate_build(json)),
    ),
];

/// Check the build count, then validate each build; errors name the build.
fn validate_all<P>(
    game: &str,
    builds: &[&str],
    min: usize,
    max: usize,
    validate: fn(&str) -> Result<Valid<P>, Vec<BuildError>>,
) -> Result<Vec<Valid<P>>, String> {
    if builds.len() < min || builds.len() > max {
        let want = if min == max {
            format!("{min}")
        } else {
            format!("{min} to {max}")
        };
        return Err(format!("{game} takes {want} builds, got {}", builds.len()));
    }
    builds
        .iter()
        .enumerate()
        .map(|(i, b)| {
            validate(b).map_err(|e| format!("builds[{i}] is invalid: {}", errors_json(&e)))
        })
        .collect()
}

/// The levels of a validated build, in the catalog's stat order.
fn stat_levels<const N: usize>(levels: &Levels, keys: [&str; N]) -> [u8; N] {
    keys.map(|k| levels.get(k).unwrap_or(0))
}

impl Game for TankRules {
    const MIN_AGENTS: usize = 2;
    const MAX_AGENTS: usize = 2;

    fn agent_name(agent: usize) -> String {
        let team = if agent == 0 { "blue" } else { "orange" };
        format!("{team}_{agent}")
    }

    /// A duel (`tank::rules::duel`): `builds[0]` is Blue, `builds[1]` is Orange, the
    /// same as `MatchSpec` and the viewer's `fromBuilds`.
    fn lineup(builds: &[&str]) -> Result<Lineup<Self>, String> {
        let valid = validate_all(
            Self::GAME,
            builds,
            Self::MIN_AGENTS,
            Self::MAX_AGENTS,
            tank::catalog::validate_build,
        )?;
        let loadout = |v: &Valid<_>| {
            let [a, s, d] = stat_levels(&v.build.levels, tank::catalog::STAT_KEYS);
            tank::Loadout::new(a, s, d).map_err(|e| e.to_string())
        };
        let setup = tank::rules::duel(loadout(&valid[0])?, loadout(&valid[1])?);
        Ok(Lineup {
            config: setup.config,
            behaviors: valid.iter().map(Valid::scripted).collect(),
        })
    }

    /// `tank::Behavior::build` with the match seed XOR the side's salt, as in
    /// `MatchSpec::policies`.
    fn scripted(_: &Self::Config, agent: usize, id: &'static str, seed: u64) -> Box<dyn Policy> {
        let salt = [tank::matchup::BLUE_SALT, tank::matchup::ORANGE_SALT][agent];
        tank::Behavior::ALL
            .into_iter()
            .find(|b| b.key() == id)
            .expect("validated scripted id")
            .build(seed ^ salt)
    }
}

impl Game for RacingRules {
    const MIN_AGENTS: usize = 1;
    const MAX_AGENTS: usize = racing::MAX_CARS;

    fn agent_name(agent: usize) -> String {
        format!("car_{agent}")
    }

    /// A Ring race (`RacingConfig::ring_setups`): car `i` plays `builds[i]`. The grid
    /// is shuffled by the match seed.
    fn lineup(builds: &[&str]) -> Result<Lineup<Self>, String> {
        let valid = validate_all(
            Self::GAME,
            builds,
            Self::MIN_AGENTS,
            Self::MAX_AGENTS,
            racing::catalog::validate_build,
        )?;
        let setups: Vec<_> = valid
            .iter()
            .map(|v| racing::catalog::setup(&v.build.levels))
            .collect();
        let config = racing::RacingConfig::ring_setups(&setups);
        config.validate().map_err(|e| e.to_string())?;
        Ok(Lineup {
            config,
            behaviors: valid.iter().map(Valid::scripted).collect(),
        })
    }

    /// `racing::drivers::Behavior::driver`, as in `racing::balance::run`.
    fn scripted(
        config: &Self::Config,
        agent: usize,
        id: &'static str,
        seed: u64,
    ) -> Box<dyn Policy<Self>> {
        racing::drivers::Behavior::from_key(id)
            .expect("validated scripted id")
            .driver(config, agent, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_match_the_crates() {
        assert_eq!(GAMES.map(|g| g.id), ["tank", "racing"]);
        assert_eq!(GAMES[0].obs_len, 176);
        assert_eq!(GAMES[0].action_len, 4);
        assert_eq!(GAMES[1].obs_len, 43);
        assert_eq!(GAMES[1].action_len, 2);
        assert!((GAMES[1].validate_json)(GAMES[1].default_build_json).starts_with(r#"{"ok":true"#));
        assert!((GAMES[0].validate_json)("{}").starts_with(r#"{"ok":false"#));
    }

    #[test]
    fn tank_lineup_is_the_matchspec_duel() {
        let b = r#"{"rules_version":1,"levels":{"attack":5,"speed":3,"defense":1},"behavior":{"kind":"scripted","id":"kiter"}}"#;
        let o = r#"{"rules_version":1,"levels":{"attack":4,"speed":1,"defense":4},"behavior":{"kind":"champion","ref":"gen-3"}}"#;
        let l = TankRules::lineup(&[b, o]).unwrap();
        let spec =
            tank::MatchSpec::from_query("seed=1&blue=kiter-5-3-1&orange=charger-4-1-4").unwrap();
        assert_eq!(l.config, spec.setup().config);
        assert_eq!(l.behaviors, [Some("kiter"), None]);
        let e = TankRules::lineup(&[b]).unwrap_err();
        assert_eq!(e, "tank takes 2 builds, got 1");
        let bad = r#"{"rules_version":1,"levels":{"attack":5,"speed":3,"defense":3},"behavior":{"kind":"scripted","id":"kiter"}}"#;
        let e = TankRules::lineup(&[b, bad]).unwrap_err();
        assert!(
            e.starts_with("builds[1] is invalid: [{\"code\":\"over_budget\""),
            "{e}"
        );
    }

    #[test]
    fn racing_lineup_takes_one_to_four_cars() {
        let d = racing::catalog::DEFAULT_BUILD_JSON;
        assert_eq!(RacingRules::lineup(&[d]).unwrap().config.cars.len(), 1);
        let l = RacingRules::lineup(&[d; 4]).unwrap();
        assert_eq!(
            l.config,
            racing::RacingConfig::ring_setups(&[racing::Setup::BALANCED; 4])
        );
        assert_eq!(l.behaviors, [Some("follower"); 4]);
        assert_eq!(
            RacingRules::lineup(&[]).unwrap_err(),
            "racing takes 1 to 4 builds, got 0"
        );
        assert!(RacingRules::lineup(&[d; 5]).is_err());
        assert!(RacingRules::lineup(&[tank::catalog::DEFAULT_BUILD_JSON]).is_err());
    }
}
