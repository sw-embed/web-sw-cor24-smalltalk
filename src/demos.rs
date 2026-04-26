/// A demo entry shown in the dropdown.
///
/// `smalltalk` is the canonical `.st` source from
/// `../sw-cor24-smalltalk/examples/`, displayed in the source pane.
/// `runtime` is the bundle the WASM runner actually executes (image
/// from `tools/stc.awk` + `vm.bas` + the BASIC top-level driver).
/// BASIC is implementation substrate — not surfaced in the UI.
pub struct Demo {
    pub name: &'static str,
    pub description: &'static str,
    pub smalltalk: &'static str,
    pub runtime: &'static str,
    pub interactive: bool,
}

pub static DEMOS: &[Demo] = &[
    Demo {
        name: "repl",
        description: "Interactive integer calculator: receiver, selector, argument.",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d5_calc.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/repl.bas")),
        interactive: true,
    },
    Demo {
        name: "hello",
        description: "Transcript show: 'hello, world' \u{2014} smallest v1-dialect demo (strings)",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/hello.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/hello.bas")),
        interactive: false,
    },
    Demo {
        name: "add",
        description: "3 + 4 \u{2192} 7 via primitive SmallInteger>>+",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d1_add.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/add.bas")),
        interactive: false,
    },
    Demo {
        name: "counter",
        description: "Counter init/incr/value \u{2192} 2 (nested user\u{2192}primitive sends)",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d2_counter.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/counter.bas")),
        interactive: false,
    },
    Demo {
        name: "boolean",
        description: "5 < 10 ifTrue: 42 ifFalse: 0 \u{2192} 42 (True/False polymorphism)",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d3_boolean.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/boolean.bas")),
        interactive: false,
    },
    Demo {
        name: "max",
        description: "5 max: 3 \u{2192} 5; real JUMP_IF_FALSE inside a Smalltalk method",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d4_max.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/max.bas")),
        interactive: false,
    },
    Demo {
        name: "factorial",
        description: "5 fact \u{2192} 120 via recursive SmallInteger>>fact",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d6_fact.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/factorial.bas")),
        interactive: false,
    },
    Demo {
        name: "bounded",
        description: "BoundedCounter caps at 5 \u{2014} first inheritance demo",
        smalltalk: include_str!("../../sw-cor24-smalltalk/examples/d7_bounded.st"),
        runtime: include_str!(concat!(env!("OUT_DIR"), "/bounded.bas")),
        interactive: false,
    },
];

pub fn default_demo_index() -> usize {
    DEMOS.iter().position(|d| d.name == "repl").unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::Session;

    fn find(name: &str) -> &'static Demo {
        DEMOS
            .iter()
            .find(|d| d.name == name)
            .unwrap_or_else(|| panic!("no demo named {name}"))
    }

    fn run_to_completion(demo: &Demo) -> Session {
        let mut s = Session::new(demo.runtime);
        for _ in 0..1_000_000 {
            if s.tick().done {
                break;
            }
        }
        s
    }

    // ---- catalog invariants -------------------------------------------------

    #[test]
    fn default_is_repl() {
        assert_eq!(DEMOS[default_demo_index()].name, "repl");
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = DEMOS.iter().map(|d| d.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "duplicate demo names");
    }

    #[test]
    fn only_repl_is_interactive() {
        for d in DEMOS {
            let expected = d.name == "repl";
            assert_eq!(
                d.interactive, expected,
                "demo {:?} interactive flag should be {expected}",
                d.name,
            );
        }
    }

    // ---- end-to-end through the WASM runner --------------------------------
    //
    // Catches breakage like the silent dual-driver bug where the live demo
    // displayed compiler-generated .st but executed the legacy hand-assembly.
    // Each test runs the bundle the browser actually loads.

    #[test]
    fn hello_prints_hello_world() {
        let s = run_to_completion(find("hello"));
        assert!(s.is_halted(), "hello did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains("hello, world"),
            "hello output missing greeting: {:?}",
            s.output()
        );
    }

    #[test]
    fn add_prints_7() {
        let s = run_to_completion(find("add"));
        assert!(s.is_halted(), "add did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains('7'),
            "add output missing 7: {:?}",
            s.output()
        );
    }

    #[test]
    fn counter_prints_2() {
        let s = run_to_completion(find("counter"));
        assert!(s.is_halted(), "counter did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains('2'),
            "counter output missing 2: {:?}",
            s.output()
        );
    }

    #[test]
    fn boolean_prints_42() {
        let s = run_to_completion(find("boolean"));
        assert!(s.is_halted(), "boolean did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains("42"),
            "boolean output missing 42: {:?}",
            s.output()
        );
    }

    #[test]
    fn max_prints_5() {
        let s = run_to_completion(find("max"));
        assert!(s.is_halted(), "max did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains('5'),
            "max output missing 5: {:?}",
            s.output()
        );
    }

    #[test]
    fn factorial_prints_120() {
        let s = run_to_completion(find("factorial"));
        assert!(s.is_halted(), "factorial did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains("120"),
            "factorial output missing 120: {:?}",
            s.output()
        );
    }

    #[test]
    fn bounded_prints_5() {
        let s = run_to_completion(find("bounded"));
        assert!(s.is_halted(), "bounded did not halt: {}", s.stop_reason());
        assert!(
            s.output().contains('5'),
            "bounded output missing 5: {:?}",
            s.output()
        );
    }

    #[test]
    fn repl_pauses_for_input() {
        let d = find("repl");
        assert!(d.interactive, "repl should be interactive");
        let mut s = Session::new_interactive(d.runtime);
        for _ in 0..1_000_000 {
            let r = s.tick();
            if r.done || s.is_awaiting_input() {
                break;
            }
        }
        assert!(
            s.is_awaiting_input(),
            "repl did not pause for input: stop={:?} out={:?}",
            s.stop_reason(),
            s.output()
        );
    }
}
