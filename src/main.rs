use shrs::prelude::*;
use shrs::readline::painter::Painter;
use shrs_autocd::AutocdPlugin;
use shrs_cd_stack::{cd_stack_down, cd_stack_up, CdStackPlugin};
use shrs_cd_tools::git;
use shrs_command_timer::{CommandTimerPlugin, CommandTimerState};
use shrs_file_history::FileBackedHistoryPlugin;
use shrs_file_logger::{FileLogger, LevelFilter};
use shrs_mux::{python::*, BashLang, MuxHighlighter, MuxPlugin, MuxState, NuLang};
use shrs_rhai::RhaiPlugin;
use shrs_rhai_completion::CompletionsPlugin;
use shrs_run_context::RunContextPlugin;
use std::{
    fs,
    io::{BufWriter, Stdout},
    path::PathBuf,
    process::{Command, Termination},
    rc::Rc,
};

pub struct Startup {
    envs: Env,
    ales: Alias,
    bilt: Rc<Builtins>,
    bind: Keybindings,
    comp: LavishCompleter,
    menu: LavishMenu,
    snip: Snippets,
    hook: Hooks,
    prom: Prompt,
}

struct LavishCompleter(DefaultCompleter);

impl Completer for LavishCompleter {
    fn complete(&self, ctx: &CompletionCtx) -> Vec<Completion> {
        <DefaultCompleter as Completer>::complete(&self.0, ctx)
    }
    fn register(&mut self, rule: Rule) {
        <DefaultCompleter as Completer>::register(&mut self.0, rule)
    }
}

impl LavishCompleter {
    fn builder() -> Builder {
        Builder::default()
    }
}

#[doc(hidden)]
struct Builder {
    completer: DefaultCompleter,
    rules: Vec<Rule>,
}

#[doc(hidden)]
impl Builder {
    fn reg(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }
    fn build(mut self) -> LavishCompleter {
        if self.rules.len() < 1 {
            return LavishCompleter(self.completer);
        }
        for rule in self.rules {
            self.completer.register(rule);
        }
        LavishCompleter(self.completer)
    }
}

#[doc(hidden)]
impl Default for Builder {
    fn default() -> Self {
        Builder {
            completer: DefaultCompleter::default(),
            rules: Vec::with_capacity(16),
        }
    }
}

struct LavishMenu {}

type Out = BufWriter<Stdout>;

impl Menu for LavishMenu {
    type MenuItem = Mesh;
    type PreviewItem = MeshPreview;
    fn next(&mut self) {
        todo!()
    }

    fn previous(&mut self) {
        todo!()
    }

    fn accept(&mut self) -> Option<&Self::MenuItem> {
        todo!()
    }

    fn is_active(&self) -> bool {
        todo!()
    }

    fn current_selection(&self) -> Option<&Self::MenuItem> {
        todo!()
    }

    fn cursor(&self) -> u32 {
        todo!()
    }

    fn activate(&mut self) {
        todo!()
    }

    // TODO: Open issue with renaming on trait to
    // fix naming into `reactivate`
    fn disactivate(&mut self) {
        todo!()
    }

    // TODO: See how easily it is to tweak this into `-> impl IntoIter<...>`
    fn items(&self) -> Vec<&(Self::PreviewItem, Self::MenuItem)> {
        todo!()
    }

    fn set_items(&mut self, mut items: Vec<(Self::PreviewItem, Self::MenuItem)>) {
        todo!()
    }

    fn render(&self, out: &mut Out, painter: &Painter) -> anyhow::Result<()> {
        todo!()
    }

    fn required_lines(&self, painter: &Painter) -> usize {
        todo!()
    }
}

/// Utility to truncate string and insert ellipses at end
fn truncate(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_string(),
        Some((idx, _)) => {
            let mut truncated = s[..idx.saturating_sub(3)].to_string();
            truncated.push_str("...");
            truncated
        }
    }
}

#[derive(Clone)]
struct Mesh(Completion);

impl AsRef<Completion> for Mesh {
    fn as_ref(&self) -> &Completion {
        &self.0
    }
}

impl AsMut<Completion> for Mesh {
    fn as_mut(&mut self) -> &mut Completion {
        &mut self.0
    }
}

use std::fmt::{Display, Formatter, Result as FmtRes};

#[derive(Debug, Clone)]
struct MeshPreview(String);

impl Display for MeshPreview {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        write!(f, "{}", self.0)
    }
}

#[allow(unused)]
#[rustfmt::skip]
fn main() -> anyhow::Result<()> {
    let envs: Env = startup::override_environment()?;
    let ales = startup::override_aliases();
    let bilt = startup::override_builtins();
    let keys = startup::override_keybinds();
    let prom = startup::override_prompt();
    let comp = startup::override_completer(&bilt, &envs);

    use std::sync::atomic::{Ordering::{Relaxed, Acquire}, AtomicBool, AtomicPtr};
    use std::thread;  
    
    let exit = &AtomicBool::new(false);
    let refr = &AtomicBool::new(false);
    let halt = &AtomicBool::new(false);

    let main_thread = thread::current();
    
    thread::scope(|mom| {
        
        mom.spawn(move || {
            let lavish = ShellBuilder::default().build().unwrap();
            lavish.run();            
        });
    });
    
    Ok(())
}

pub fn run_shell(startup: Startup) -> anyhow::Result<()> {
    ShellBuilder::default()
        .with_completer(startup.comp)
        .with_hooks(startup.hook)
        .with_env(startup.envs)
        .with_alias(startup.ales)
        .with_keybindings(startup.bind)
        .with_builtins(Rc::into_inner(startup.bilt).expect("..."))
        .with_prompt(startup.prom)
        .with_plugin(CommandTimerPlugin)
        .with_plugin(RunContextPlugin::default())
        .with_plugin(
            MuxPlugin::new()
                .register_lang("bash", BashLang::new())
                .register_lang("python", PythonLang::new())
                .register_theme("python", Box::new(PythonTheme::new()))
                .register_lang("nu", NuLang::new()),
        )
        .with_plugin(CdStackPlugin)
        .with_plugin(RhaiPlugin)
        .with_plugin(CompletionsPlugin)
        .with_plugin(FileBackedHistoryPlugin::new())
        .with_plugin(AutocdPlugin)
        .build()
        .unwrap()
        .run()
}

#[allow(unused_imports)]
pub mod lsh {
    use std::thread;
}

#[allow(unused_imports)]
mod startup {
    use super::*;
    use std::cell::RefCell;
    use std::thread;

    pub(crate) fn override_aliases() -> Alias {
        Alias::new()
    }

    pub(crate) fn override_keybinds() -> Keybindings {
        Keybindings::new()
    }

    pub(crate) fn override_builtins() -> Rc<Builtins> {
        Rc::new(Builtins::default())
    }

    /// Prepare an environment which assigns prcedence to runtime supplied environment variables
    /// in the .env file located in `$XDG_CONFIG_HOME/lavish/.env`, if it exists.
    /// If the process is to be a child of an existing shell session, it inherits the environment
    /// variables.
    pub(crate) fn override_environment() -> Result<Env> {
        // Load environment variables from `XDG_CONFIG_HOME/lavish/.env`
        use dirs;
        use dotenvy;
        let mut env = Env::new();
        let usr = dirs::config_local_dir();
        if let Some(r#local) = usr {
            dotenvy::from_filename(r#local.join(".env")).unwrap_or_default();
        }
        env.load().expect("OS to provide environment variables");
        Ok(env)
    }

    pub(crate) fn override_completer(builtins: &Rc<Builtins>, env: &Env) -> impl Completer {
        fn cmdname_predicate(ctx: &CompletionCtx) -> bool {
            ctx.arg_num() == 0
        }

        let builtins = Rc::clone(builtins);

        let path0 = env.get("PATH").unwrap().to_string();

        LavishCompleter::builder()
            .reg(Rule::new(
                Pred::new(cmdname_predicate),
                Box::new(cmdname_action(path0)),
            ))
            .reg(Rule::new(
                Pred::new(cmdname_predicate),
                Box::new(builtin_cmdname_action(&builtins)),
            ))
            .build()
    }

    pub(crate) fn override_prompt() -> Result<Prompt> {
        Ok(Prompt::from_sides(prompt_left, prompt_right))
    }
}

// =-=-= Prompt customization =-=-=
// Create a new struct and implement the [Prompt] trait
fn prompt_left(line_mode: State<LineMode>, contents: State<LineContents>) -> StyledBuf {
    let indicator = match *line_mode {
        LineMode::Insert => String::from(">").cyan(),
        LineMode::Normal => String::from(":").yellow(),
    };
    if !contents.lines.is_empty() {
        return styled_buf!(" ", indicator, " ");
    }

    styled_buf!(
        " ",
        username().map(|u| u.blue()),
        " ",
        top_pwd().white().bold(),
        " ",
        indicator,
        " "
    )
}

fn prompt_right(cmd_timer: State<CommandTimerState>, mux: State<MuxState>) -> StyledBuf {
    let time_str = cmd_timer.command_time().map(|x| format!("{x:?}"));

    let lang_name = mux.current_lang().name();

    if let Ok(git_branch) = git::branch().map(|s| format!("git:{s}").blue().bold()) {
        styled_buf!(git_branch, " ", time_str, " ", lang_name, " ")
    } else {
        styled_buf!(time_str, " ", lang_name, " ")
    }
}
