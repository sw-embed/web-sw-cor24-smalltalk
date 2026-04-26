pub struct Demo {
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    pub interactive: bool,
}

/// COR24 Smalltalk v0 demos. Each `source` is a build-time concat of
/// `src/image_dN.bas + src/vm.bas + examples/dN_*.bas` from the sibling
/// `sw-cor24-smalltalk` repo, with trailing `RUN`/`BYE` REPL commands
/// stripped (the runner appends them based on `interactive`).
pub static DEMOS: &[Demo] = &[
    Demo {
        name: "d1_add",
        description: "3 + 4 \u{2192} 7 via primitive SmallInteger>>+",
        source: include_str!(concat!(env!("OUT_DIR"), "/d1_add.bas")),
        interactive: false,
    },
    Demo {
        name: "d2_counter",
        description: "Counter init/incr/value \u{2192} 2; nested user\u{2192}primitive sends",
        source: include_str!(concat!(env!("OUT_DIR"), "/d2_counter.bas")),
        interactive: false,
    },
    Demo {
        name: "d3_boolean",
        description: "5 < 10 ifTrue: 42 ifFalse: 0 \u{2192} 42 via True/False polymorphism",
        source: include_str!(concat!(env!("OUT_DIR"), "/d3_boolean.bas")),
        interactive: false,
    },
    Demo {
        name: "d4_max",
        description: "5 max: 3 \u{2192} 5; real JUMP_IF_FALSE inside a user method",
        source: include_str!(concat!(env!("OUT_DIR"), "/d4_max.bas")),
        interactive: false,
    },
    Demo {
        name: "d5_calc",
        description: "Integer calc REPL \u{2014} enter A, OP, B (1=+ 2=- 3=* 4=< 14=max:; 0=quit)",
        source: include_str!(concat!(env!("OUT_DIR"), "/d5_calc.bas")),
        interactive: true,
    },
    Demo {
        name: "d6_fact",
        description: "5 fact \u{2192} 120 via recursive SmallInteger>>fact",
        source: include_str!(concat!(env!("OUT_DIR"), "/d6_fact.bas")),
        interactive: false,
    },
    Demo {
        name: "d7_bounded",
        description: "BoundedCounter caps at 5; first inheritance demo (superclass walk)",
        source: include_str!(concat!(env!("OUT_DIR"), "/d7_bounded.bas")),
        interactive: false,
    },
];

pub fn default_demo_index() -> usize {
    DEMOS.iter().position(|d| d.name == "d1_add").unwrap_or(0)
}
