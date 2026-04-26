use gloo::timers::callback::Timeout;
use web_sys::{Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, KeyboardEvent};
use yew::prelude::*;

pub mod config;
pub mod demos;
pub mod runner;

use demos::{DEMOS, default_demo_index};
use runner::Session;

const DEFAULT_MAX_INSTRS: u64 = 200_000_000;
const TICK_DELAY_MS: u32 = 0;

fn now_ms() -> f64 {
    js_sys::Date::now()
}

pub enum Msg {
    SelectDemo(usize),
    SourceChanged(String),
    Run,
    Tick,
    Stop,
    Reset,
    Clear,
    IncreaseBudget,
    KeyDown(KeyboardEvent),
    InputChanged(String),
    InputSubmit,
}

pub struct App {
    selected: usize,
    source: String,
    output: String,
    status: String,
    error: bool,
    session: Option<Session>,
    running: bool,
    max_instrs: u64,
    started_at: f64,
    elapsed_ms: f64,
    budget_exhausted: bool,
    input_line: String,
    awaiting_input: bool,
    output_ref: NodeRef,
    input_ref: NodeRef,
}

impl App {
    fn load_demo(&mut self, idx: usize) {
        if let Some(demo) = DEMOS.get(idx) {
            self.selected = idx;
            self.source = demo.smalltalk.to_string();
            self.output.clear();
            self.status = "idle".into();
            self.error = false;
            self.session = None;
            self.running = false;
            self.budget_exhausted = false;
            self.elapsed_ms = 0.0;
            self.input_line.clear();
            self.awaiting_input = false;
        }
    }

    fn start_run(&mut self, ctx: &Context<Self>) {
        let Some(demo) = DEMOS.get(self.selected) else {
            return;
        };
        self.session = Some(if demo.interactive {
            Session::new_interactive(demo.runtime)
        } else {
            Session::new(demo.runtime)
        });
        self.input_line.clear();
        self.awaiting_input = false;
        self.running = true;
        self.error = false;
        self.budget_exhausted = false;
        self.output.clear();
        self.started_at = now_ms();
        self.elapsed_ms = 0.0;
        self.status = "running...".into();
        self.schedule_tick(ctx);
    }

    fn schedule_tick(&self, ctx: &Context<Self>) {
        let link = ctx.link().clone();
        Timeout::new(TICK_DELAY_MS, move || link.send_message(Msg::Tick)).forget();
    }

    fn finish(&mut self, status: String, error: bool) {
        self.running = false;
        self.status = status;
        self.error = error;
        self.elapsed_ms = now_ms() - self.started_at;
        if let Some(s) = &self.session {
            let raw = s.output();
            self.output = raw;
        }
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let idx = default_demo_index();
        let demo = &DEMOS[idx];
        Self {
            selected: idx,
            source: demo.smalltalk.to_string(),
            output: String::new(),
            status: "idle".into(),
            error: false,
            session: None,
            running: false,
            max_instrs: DEFAULT_MAX_INSTRS,
            started_at: 0.0,
            elapsed_ms: 0.0,
            budget_exhausted: false,
            input_line: String::new(),
            awaiting_input: false,
            output_ref: NodeRef::default(),
            input_ref: NodeRef::default(),
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, _first_render: bool) {
        if let Some(el) = self.output_ref.cast::<Element>() {
            el.set_scroll_top(el.scroll_height());
        }
        if self.awaiting_input
            && let Some(el) = self.input_ref.cast::<HtmlInputElement>()
        {
            let _ = el.focus();
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectDemo(i) => {
                self.load_demo(i);
                self.max_instrs = DEFAULT_MAX_INSTRS;
                true
            }
            Msg::SourceChanged(v) => {
                self.source = v;
                false
            }
            Msg::Run => {
                self.max_instrs = DEFAULT_MAX_INSTRS;
                self.start_run(ctx);
                true
            }
            Msg::IncreaseBudget => {
                self.max_instrs = self.max_instrs.saturating_mul(4);
                self.start_run(ctx);
                true
            }
            Msg::Stop => {
                if self.running {
                    self.finish("stopped".into(), false);
                }
                true
            }
            Msg::Reset => {
                let idx = self.selected;
                self.running = false;
                self.session = None;
                self.load_demo(idx);
                self.max_instrs = DEFAULT_MAX_INSTRS;
                true
            }
            Msg::Clear => {
                self.output.clear();
                if !self.running {
                    self.status = "idle".into();
                    self.error = false;
                    self.budget_exhausted = false;
                }
                true
            }
            Msg::Tick => {
                if !self.running {
                    return false;
                }
                let Some(session) = self.session.as_mut() else {
                    self.running = false;
                    return true;
                };
                let interactive = DEMOS
                    .get(self.selected)
                    .map(|d| d.interactive)
                    .unwrap_or(false);
                if !interactive {
                    let remaining = self.max_instrs.saturating_sub(session.instructions());
                    if remaining == 0 {
                        self.budget_exhausted = true;
                        let instrs = session.instructions();
                        self.finish(format!("halted (budget) -- {} instrs", instrs), true);
                        return true;
                    }
                }
                let result = session.tick();
                if session.is_awaiting_input() {
                    self.awaiting_input = true;
                    self.output = session.output();
                    self.elapsed_ms = now_ms() - self.started_at;
                    self.status = format!(
                        "awaiting input ({} instrs, {:.0} ms)",
                        session.instructions(),
                        self.elapsed_ms
                    );
                    return true;
                }
                if result.done {
                    let instrs = session.instructions();
                    let reason = session.stop_reason();
                    let halted = session.is_halted();
                    self.finish(
                        format!(
                            "{} ({} instrs, {:.0} ms)",
                            reason,
                            instrs,
                            now_ms() - self.started_at
                        ),
                        !halted,
                    );
                } else {
                    self.output = session.output();
                    self.elapsed_ms = now_ms() - self.started_at;
                    self.status = format!(
                        "running... {} instrs, {:.0} ms",
                        session.instructions(),
                        self.elapsed_ms
                    );
                    self.schedule_tick(ctx);
                }
                true
            }
            Msg::InputChanged(v) => {
                self.input_line = v;
                false
            }
            Msg::InputSubmit => {
                if !self.awaiting_input {
                    return false;
                }
                let line = std::mem::take(&mut self.input_line);
                if let Some(session) = self.session.as_mut() {
                    session.feed_input(&line);
                }
                self.awaiting_input = false;
                self.status = "running...".into();
                self.schedule_tick(ctx);
                true
            }
            Msg::KeyDown(e) => {
                if e.key() == "Enter" && (e.ctrl_key() || e.meta_key()) {
                    e.prevent_default();
                    ctx.link().send_message(Msg::Run);
                } else if e.key() == "Escape" && self.running {
                    e.prevent_default();
                    ctx.link().send_message(Msg::Stop);
                }
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let on_demo = ctx.link().callback(|e: Event| {
            let target: HtmlSelectElement = e.target_unchecked_into();
            let idx: usize = target.value().parse().unwrap_or(0);
            Msg::SelectDemo(idx)
        });
        let on_src = ctx.link().callback(|e: InputEvent| {
            let target: HtmlTextAreaElement = e.target_unchecked_into();
            Msg::SourceChanged(target.value())
        });
        let on_run = ctx.link().callback(|_| Msg::Run);
        let on_stop = ctx.link().callback(|_| Msg::Stop);
        let on_reset = ctx.link().callback(|_| Msg::Reset);
        let on_clear = ctx.link().callback(|_| Msg::Clear);
        let on_inc = ctx.link().callback(|_| Msg::IncreaseBudget);
        let on_keydown = ctx.link().callback(Msg::KeyDown);
        let on_input_change = ctx.link().callback(|e: InputEvent| {
            let target: HtmlInputElement = e.target_unchecked_into();
            Msg::InputChanged(target.value())
        });
        let on_input_submit = ctx.link().callback(|_| Msg::InputSubmit);
        let on_input_keydown = ctx.link().callback(|e: KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                Msg::InputSubmit
            } else {
                Msg::KeyDown(e)
            }
        });

        let status_class = if self.error {
            "status status-error"
        } else {
            "status"
        };
        let run_button = if self.running {
            html! { <button onclick={on_stop}>{ "Stop" }</button> }
        } else {
            html! { <button onclick={on_run}>{ "Run" }</button> }
        };

        html! {
            <>
            <a href="https://github.com/sw-embed/web-sw-cor24-smalltalk" class="github-corner"
               aria-label="View source on GitHub" target="_blank">
                <svg width="80" height="80" viewBox="0 0 250 250" aria-hidden="true">
                    <path d="M0,0 L115,115 L130,115 L142,142 L250,250 L250,0 Z" />
                    <path d="M128.3,109.0 C113.8,99.7 119.0,89.6 119.0,89.6 C122.0,82.7 120.5,78.6 \
                        120.5,78.6 C119.2,72.0 123.4,76.3 123.4,76.3 C127.3,80.9 125.5,87.3 125.5,87.3 \
                        C122.9,97.6 130.6,101.9 134.4,103.2" fill="currentColor"
                        style="transform-origin:130px 106px;" class="octo-arm" />
                    <path d="M115.0,115.0 C114.9,115.1 118.7,116.5 119.8,115.4 L133.7,101.6 C136.9,99.2 \
                        139.9,98.4 142.2,98.6 C133.8,88.0 127.5,74.4 143.8,58.0 C148.5,53.4 154.0,51.2 \
                        159.7,51.0 C160.3,49.4 163.2,43.6 171.4,40.1 C171.4,40.1 176.1,42.5 178.8,56.2 \
                        C183.1,58.6 187.2,61.8 190.9,65.4 C194.5,69.0 197.7,73.2 200.1,77.6 C213.8,80.2 \
                        216.3,84.9 216.3,84.9 C212.7,93.1 206.9,96.0 205.4,96.6 C205.1,102.4 203.0,107.8 \
                        198.3,112.5 C181.9,128.9 168.3,122.5 157.7,114.1 C157.9,116.9 156.7,120.9 \
                        152.7,124.9 L141.0,136.5 C139.8,137.7 141.6,141.9 141.8,141.8 Z"
                        fill="currentColor" />
                </svg>
            </a>
            <main class="page" onkeydown={on_keydown.clone()}>
                <header class="chrome">
                    <h1>{ "web-sw-cor24-smalltalk" }</h1>
                    <div class="controls">
                        <select onchange={on_demo} disabled={self.running}>
                            { for DEMOS.iter().enumerate().map(|(i, d)| html! {
                                <option value={i.to_string()} selected={i == self.selected}
                                        title={d.description}>
                                    { d.name }
                                </option>
                            })}
                        </select>
                        { run_button }
                        <button class="secondary" onclick={on_reset} disabled={self.running}>{ "Reset" }</button>
                        <button class="secondary" onclick={on_clear}>{ "Clear" }</button>
                    </div>
                </header>
                <div class="workspace">
                <section class="panel panel-src">
                    <label>{ "smalltalk" }</label>
                    <textarea
                        class="src"
                        spellcheck="false"
                        readonly={true}
                        value={self.source.clone()}
                        oninput={on_src}
                        onkeydown={on_keydown.clone()}
                    />
                </section>
                <section class="panel panel-out">
                    <div class={status_class}>
                        { format!("status: {}", self.status) }
                        { if self.budget_exhausted {
                            html! {
                                <>
                                    { " -- " }
                                    <button class="link-btn" onclick={on_inc}>
                                        { "Increase budget 4x" }
                                    </button>
                                </>
                            }
                        } else { html! {} }}
                    </div>
                    <pre class="out" ref={self.output_ref.clone()}>{ &self.output }</pre>
                    { if self.awaiting_input {
                        html! {
                            <div class="input-row">
                                <label>{ "input:" }</label>
                                <input
                                    ref={self.input_ref.clone()}
                                    type="text"
                                    value={self.input_line.clone()}
                                    oninput={on_input_change}
                                    onkeydown={on_input_keydown}
                                    autofocus=true
                                />
                                <button onclick={on_input_submit}>{ "Send" }</button>
                            </div>
                        }
                    } else { html! {} }}
                </section>
                </div>
            </main>
            <footer>
                <span>{"MIT License"}</span>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <span>{"\u{00a9} 2026 Michael A Wright"}</span>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <a href="https://makerlisp.com" target="_blank">{"COR24-TB"}</a>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <a href="https://software-wrighter-lab.github.io/" target="_blank">{"Blog"}</a>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <a href="https://discord.com/invite/Ctzk5uHggZ" target="_blank">{"Discord"}</a>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <a href="https://www.youtube.com/@SoftwareWrighter" target="_blank">{"YouTube"}</a>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <a href="https://github.com/sw-embed/sw-cor24-smalltalk" target="_blank">{"Smalltalk Source"}</a>
                <span class="footer-sep">{"\u{00b7}"}</span>
                <span>{ format!("{} \u{00b7} {} \u{00b7} {}",
                    env!("BUILD_HOST"),
                    env!("BUILD_SHA"),
                    env!("BUILD_TIMESTAMP"),
                ) }</span>
            </footer>
            </>
        }
    }
}
