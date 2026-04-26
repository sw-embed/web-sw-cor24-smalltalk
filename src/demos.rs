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
