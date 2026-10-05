//! Log level rules, parsed from `RUST_LOG` with the same syntax and matching as `env_logger`, and changed at runtime.

use std::collections::BTreeMap;

use log::LevelFilter;

/// Per-target log levels.
///
/// Like `env_logger`, a rule's target matches any target it is a prefix of, and the longest matching rule wins.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelRules {
    /// Applies to targets that don't match any target rule.
    global: Option<LevelFilter>,
    targets: BTreeMap<String, LevelFilter>,
}

#[derive(Debug, Default, PartialEq)]
pub struct Spec {
    pub rules: LevelRules,
    /// The message filter, the part of the spec after the `/`.
    pub filter: Option<String>,
    /// Problems with the spec, the affected directives are ignored.
    pub errors: Vec<String>,
}

impl LevelRules {
    pub fn with_global(level: LevelFilter) -> Self {
        Self {
            global: Some(level),
            targets: BTreeMap::new(),
        }
    }

    /// Parses a `RUST_LOG` style spec, e.g. `warn,server_cli=debug,ergot::net_stack=off/claim`.
    pub fn parse(spec: &str) -> Spec {
        let mut result = Spec::default();

        let mut parts = spec.split('/');
        let directives = parts.next().unwrap_or_default();
        let filter = parts.next();
        if parts.next().is_some() {
            result
                .errors
                .push(format!("invalid logging spec '{}' (too many '/'s), ignoring it", spec));
            return result;
        }
        result.filter = filter
            .filter(|filter| !filter.is_empty())
            .map(str::to_string);

        for directive in directives.split(',').map(str::trim) {
            if directive.is_empty() {
                continue;
            }
            let mut parts = directive.split('=');
            let name = parts.next().unwrap_or_default().trim();
            match (parts.next().map(str::trim), parts.next()) {
                // a single level is the global level, anything else is a target with all logging enabled
                (None, _) => match name.parse::<LevelFilter>() {
                    Ok(level) => result.rules.global = Some(level),
                    Err(_) => result
                        .rules
                        .set(Some(name), Some(LevelFilter::Trace)),
                },
                (Some(""), None) => result
                    .rules
                    .set(Some(name), Some(LevelFilter::Trace)),
                (Some(level), None) => match level.parse::<LevelFilter>() {
                    Ok(level) if name.is_empty() => result.rules.global = Some(level),
                    Ok(level) => result
                        .rules
                        .set(Some(name), Some(level)),
                    Err(_) => result
                        .errors
                        .push(format!("invalid logging spec '{}', ignoring it", level)),
                },
                (Some(_), Some(_)) => result
                    .errors
                    .push(format!("invalid logging spec '{}', ignoring it", directive)),
            }
        }

        result
    }

    /// The level that applies to `target`.
    pub fn level_for(&self, target: &str) -> LevelFilter {
        let rule = self
            .targets
            .iter()
            .filter(|(prefix, _)| target.starts_with(prefix.as_str()))
            .max_by_key(|(prefix, _)| prefix.len())
            .map(|(_, level)| *level);

        match rule.or(self.global) {
            Some(level) => level,
            // like `env_logger`, errors are logged when there are no rules at all
            None if self.targets.is_empty() => LevelFilter::Error,
            None => LevelFilter::Off,
        }
    }

    /// The most verbose level of any rule, records above this level are never logged.
    pub fn max_level(&self) -> LevelFilter {
        if self.global.is_none() && self.targets.is_empty() {
            return LevelFilter::Error;
        }
        self.targets
            .values()
            .copied()
            .chain(self.global)
            .max()
            .unwrap_or(LevelFilter::Off)
    }

    /// The rule for `target`, `None` for the global rule.
    pub fn rule(&self, target: Option<&str>) -> Option<LevelFilter> {
        match target {
            None => self.global,
            Some(target) => self.targets.get(target).copied(),
        }
    }

    /// Sets or, with a `level` of `None`, removes the rule for `target`, `None` for the global rule.
    pub fn set(&mut self, target: Option<&str>, level: Option<LevelFilter>) {
        match (target, level) {
            (None, level) => self.global = level,
            (Some(target), Some(level)) => {
                self.targets
                    .insert(target.to_string(), level);
            }
            (Some(target), None) => {
                self.targets.remove(target);
            }
        }
    }

    /// The targets that have rules.
    pub fn targets(&self) -> impl Iterator<Item = &str> {
        self.targets.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_level() {
        let spec = LevelRules::parse("info");
        assert_eq!(spec.rules, LevelRules::with_global(LevelFilter::Info));
        assert_eq!(spec.filter, None);
        assert!(spec.errors.is_empty());
    }

    #[test]
    fn levels_are_case_insensitive() {
        let spec = LevelRules::parse("DEBUG,a=WaRn");
        assert_eq!(spec.rules.level_for("b"), LevelFilter::Debug);
        assert_eq!(spec.rules.level_for("a"), LevelFilter::Warn);
    }

    #[test]
    fn targets_and_global() {
        let spec = LevelRules::parse("warn, server_cli=debug ,server_cli::ioboard=trace,ergot=off");
        let rules = spec.rules;
        assert_eq!(rules.level_for("other"), LevelFilter::Warn);
        assert_eq!(rules.level_for("server_cli"), LevelFilter::Debug);
        assert_eq!(rules.level_for("server_cli::operator"), LevelFilter::Debug);
        assert_eq!(rules.level_for("server_cli::ioboard::discovery"), LevelFilter::Trace);
        assert_eq!(rules.level_for("ergot::net_stack"), LevelFilter::Off);
        assert_eq!(rules.max_level(), LevelFilter::Trace);
    }

    #[test]
    fn bare_target_enables_all_levels() {
        let rules = LevelRules::parse("server_cli,ergot=").rules;
        assert_eq!(rules.level_for("server_cli::x"), LevelFilter::Trace);
        assert_eq!(rules.level_for("ergot"), LevelFilter::Trace);
    }

    #[test]
    fn unmatched_targets_are_off_without_a_global_rule() {
        let rules = LevelRules::parse("server_cli=info").rules;
        assert_eq!(rules.level_for("ergot"), LevelFilter::Off);
    }

    #[test]
    fn errors_only_without_any_rules() {
        let rules = LevelRules::parse("").rules;
        assert_eq!(rules.level_for("ergot"), LevelFilter::Error);
        assert_eq!(rules.max_level(), LevelFilter::Error);
    }

    #[test]
    fn matching_is_by_prefix_like_env_logger() {
        let rules = LevelRules::parse("server=debug").rules;
        assert_eq!(rules.level_for("server_cli"), LevelFilter::Debug);
    }

    #[test]
    fn later_directives_replace_earlier_ones() {
        let rules = LevelRules::parse("a=info,a=debug").rules;
        assert_eq!(rules.level_for("a"), LevelFilter::Debug);
    }

    #[test]
    fn filter() {
        let spec = LevelRules::parse("info/claim.*board");
        assert_eq!(spec.rules, LevelRules::with_global(LevelFilter::Info));
        assert_eq!(spec.filter.as_deref(), Some("claim.*board"));
    }

    #[test]
    fn too_many_slashes_ignores_the_spec() {
        let spec = LevelRules::parse("info/a/b");
        assert_eq!(spec.rules, LevelRules::default());
        assert_eq!(spec.filter, None);
        assert_eq!(spec.errors.len(), 1);
    }

    #[test]
    fn invalid_directives_are_ignored() {
        let spec = LevelRules::parse("a=loud,b=info,c=d=e");
        assert_eq!(spec.rules.rule(Some("a")), None);
        assert_eq!(spec.rules.rule(Some("b")), Some(LevelFilter::Info));
        assert_eq!(spec.rules.rule(Some("c")), None);
        assert_eq!(spec.errors.len(), 2);
    }

    #[test]
    fn set_and_remove() {
        let mut rules = LevelRules::with_global(LevelFilter::Warn);
        rules.set(Some("a::b"), Some(LevelFilter::Trace));
        assert_eq!(rules.level_for("a::b::c"), LevelFilter::Trace);
        assert_eq!(rules.max_level(), LevelFilter::Trace);
        rules.set(Some("a::b"), None);
        assert_eq!(rules.level_for("a::b::c"), LevelFilter::Warn);
        assert_eq!(rules.max_level(), LevelFilter::Warn);
    }
}
