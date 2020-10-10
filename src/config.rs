//! Configuration options for customizing the behavior of the provided panic
//! and error reporting hooks
use crate::{
    section::PanicMessage,
    writers::{EnvSection, WriterExt},
};
use fmt::Display;
use indenter::{indented, Format};
use std::env;
use std::fmt::Write as _;
use std::{fmt, path::PathBuf, sync::Arc};
use once_cell::sync::OnceCell;
use owo_colors::{XtermColors, OwoColorize};

// re-export so end-users don't need to depend on `owo_colors`
pub use owo_colors::{style, Style};

// XXX Needed to access styles from `impl Section for Report` and `impl PanicMessage for DefaultPanicMessage`
pub(crate) static STYLES: OnceCell<Styles> = OnceCell::new();

#[derive(Debug)]
struct InstallError;

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("could not install the BacktracePrinter as another was already installed")
    }
}

impl std::error::Error for InstallError {}

#[derive(Debug)]
struct InstallStylesError;

impl fmt::Display for InstallStylesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // XXX I feel this message is not very good – any idea for something better?
        f.write_str("could not set the provided `Styles` globally as another was already set")
    }
}

impl std::error::Error for InstallStylesError {}

#[derive(Debug)]
struct InstallColorBacktraceStylesError;

impl fmt::Display for InstallColorBacktraceStylesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // XXX I feel this message is not very good – any idea for something better?
        f.write_str("could not set the provided `Styles` via `color_backtrace::set_styles` globally as another was already set")
    }
}

impl std::error::Error for InstallColorBacktraceStylesError {}

// XXX now that we support text effects, "color scheme" doesn't seem accurate anymore. Let me know if you'd prefer a different name. The good thing about `Styles` is, that it's pretty short. "Theme" would be another good name, I just realized.

/// A struct that represents styles that should be used by `color_eyre`
#[derive(Debug, Copy, Clone, Default)]
pub struct Styles {
    pub(crate) file: Style,
    pub(crate) line_number: Style,
    pub(crate) spantrace_target: Style,
    pub(crate) spantrace_fields: Style,
    pub(crate) active_line: Style,
    pub(crate) error: Style,
    pub(crate) help_info_note: Style,
    pub(crate) help_info_warning: Style,
    pub(crate) help_info_suggestion: Style,
    pub(crate) help_info_error: Style,
    pub(crate) dependency_code: Style,
    pub(crate) crate_code: Style,
    pub(crate) code_hash: Style,
    pub(crate) panic_header: Style,
    pub(crate) panic_message: Style,
    pub(crate) panic_file: Style,
    pub(crate) panic_line_number: Style,
    pub(crate) hidden_frames: Style,
}

macro_rules! style_setters {
    ($(#[$meta:meta] $name:ident),* $(,)?) => {
        $(
            #[$meta]
            pub fn $name(mut self, style: Style) -> Self {
                self.$name = style;
                self
            }
        )*
    };
}

impl Styles {
    /// Blank styles
    pub fn new() -> Self {
        Self::default()
    }

    /// Styles for dark background. This is the default
    pub fn dark() -> Self {
        Self {
            file: style().purple(),
            line_number: style().purple(),
            active_line: style().white().bold(),
            error: style().bright_red(),
            help_info_note: style().bright_cyan(),
            help_info_warning: style().bright_yellow(),
            help_info_suggestion: style().bright_cyan(),
            help_info_error: style().bright_red(),
            dependency_code: style().green(),
            crate_code: style().bright_red(),
            code_hash: style().bright_black(),
            panic_header: style().red(),
            panic_message: style().cyan(),
            panic_file: style().purple(),
            panic_line_number: style().purple(),
            hidden_frames: style().bright_cyan(),

            // XXX The following two are specific to `color_spantrace`. See my comment there for my recommendation.
            spantrace_target: style().color(XtermColors::UserBrightRed),
            spantrace_fields: style().color(XtermColors::UserBrightCyan),
        }
    }

    // XXX it would be great, if someone with more style optimizes the light theme below. I just fixed the biggest problems, but ideally there would be darker colors (however, the standard ANSI colors don't seem to have many dark enough colors. Maybe xterm colors or RGB colors would be better (however, again, see my comment regarding xterm colors in `color_spantrace`))

    /// Styles for light backgrounds
    pub fn light() -> Self {
        Self {
            file: style().purple(),
            line_number: style().purple(),
            spantrace_target: style().red(),
            spantrace_fields: style().blue(),
            active_line: style().bold(),
            error: style().red(),
            help_info_note: style().blue(),
            help_info_warning: style().bright_red(),
            help_info_suggestion: style().blue(),
            help_info_error: style().red(),
            dependency_code: style().green(),
            crate_code: style().red(),
            code_hash: style().bright_black(),
            panic_header: style().red(),
            panic_message: style().blue(),
            panic_file: style().purple(),
            panic_line_number: style().purple(),
            hidden_frames: style().blue(),
        }
    }


    style_setters! {
        /// Styles printed paths
        file,
        /// Styles the line number of a file
        line_number,
        /// Styles the `color_spantrace` target (i.e. the module and function name, and so on)
        spantrace_target,
        // XXX is this correct?
        /// Styles fields of the `tracing` crate (in the context of `color_spantrace`, the arguments of functions and methods)
        spantrace_fields,
        /// Styles the selected line of displayed code
        active_line,
        // XXX not sure how to describe this better
        /// Styles error printed by `EyreHandler`
        error,
        /// Styles the "note" section header
        help_info_note,
        /// Styles the "warning" section header
        help_info_warning,
        /// Styles the "suggestion" section header
        help_info_suggestion,
        /// Styles the "error" section header
        help_info_error,
        /// Styles code that is not part of your crate
        dependency_code,
        /// Styles code that's in your crate
        crate_code,
        /// Styles the hash after `dependency_code` and `crate_code`
        code_hash,
        /// Styles the header of a panic
        panic_header,
        /// Styles the message of a panic
        panic_message,
        /// Styles paths of a panic
        panic_file,
        /// Styles the line numbers of a panic
        panic_line_number,
        /// Styles the "N frames hidden" message
        hidden_frames,
    }
}

/// A representation of a Frame from a Backtrace or a SpanTrace
#[derive(Debug)]
#[non_exhaustive]
pub struct Frame {
    /// Frame index
    pub n: usize,
    /// frame symbol name
    pub name: Option<String>,
    /// source line number
    pub lineno: Option<u32>,
    /// source file path
    pub filename: Option<PathBuf>,
    // XXX is it correct that this is public?
    /// The global styles that `color_eyre` uses
    pub styles: Styles,
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let is_dependency_code = self.is_dependency_code();

        // Print frame index.
        write!(f, "{:>2}: ", self.n)?;

        // Does the function have a hash suffix?
        // (dodging a dep on the regex crate here)
        let name = self.name.as_deref().unwrap_or("<unknown>");
        let has_hash_suffix = name.len() > 19
            && &name[name.len() - 19..name.len() - 16] == "::h"
            && name[name.len() - 16..].chars().all(|x| x.is_digit(16));

        // Print function name.

        if has_hash_suffix {
            if is_dependency_code {
                write!(f, "{}", (&name[..name.len() - 19]).style(self.styles.dependency_code))?;
            } else {
                write!(f, "{}", (&name[..name.len() - 19]).style(self.styles.crate_code))?;
            }
            write!(f, "{}", (&name[name.len() - 19..]).style(self.styles.code_hash))?;
        } else {
            write!(f, "{}", name)?;
        }

        let mut separated = f.header("\n");

        // Print source location, if known.
        if let Some(ref file) = self.filename {
            let filestr = file.to_str().unwrap_or("<bad utf8>");
            let lineno = self
                .lineno
                .map_or("<unknown line>".to_owned(), |x| x.to_string());
            write!(
                &mut separated.ready(),
                "    at {}:{}",
                filestr.style(self.styles.file),
                lineno.style(self.styles.line_number),
            )?;
        } else {
            write!(&mut separated.ready(), "    at <unknown source file>")?;
        }

        let v = if std::thread::panicking() {
            panic_verbosity()
        } else {
            lib_verbosity()
        };

        // Maybe print source.
        if v >= Verbosity::Full {
            write!(&mut separated.ready(), "{}", SourceSection(self))?;
        }

        Ok(())
    }
}

struct SourceSection<'a>(&'a Frame);

impl fmt::Display for SourceSection<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (lineno, filename) = match (self.0.lineno, self.0.filename.as_ref()) {
            (Some(a), Some(b)) => (a, b),
            // Without a line number and file name, we can't sensibly proceed.
            _ => return Ok(()),
        };

        let file = match std::fs::File::open(filename) {
            Ok(file) => file,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            e @ Err(_) => e.unwrap(),
        };

        use std::fmt::Write;
        use std::io::BufRead;

        // Extract relevant lines.
        let reader = std::io::BufReader::new(file);
        let start_line = lineno - 2.min(lineno - 1);
        let surrounding_src = reader.lines().skip(start_line as usize - 1).take(5);
        let mut separated = f.header("\n");
        let mut f = separated.in_progress();
        for (line, cur_line_no) in surrounding_src.zip(start_line..) {
            let line = line.unwrap();
            if cur_line_no == lineno {
                write!(
                    &mut f,
                    "{:>8} {} {}",
                    cur_line_no.style(self.0.styles.active_line),
                    ">".style(self.0.styles.active_line),
                    line.style(self.0.styles.active_line),
                )?;
            } else {
                write!(&mut f, "{:>8} │ {}", cur_line_no, line)?;
            }
            f = separated.ready();
        }

        Ok(())
    }
}

impl Frame {
    fn is_dependency_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "std::",
            "core::",
            "backtrace::backtrace::",
            "_rust_begin_unwind",
            "color_traceback::",
            "__rust_",
            "___rust_",
            "__pthread",
            "_main",
            "main",
            "__scrt_common_main_seh",
            "BaseThreadInitThunk",
            "_start",
            "__libc_start_main",
            "start_thread",
        ];

        // Inspect name.
        if let Some(ref name) = self.name {
            if SYM_PREFIXES.iter().any(|x| name.starts_with(x)) {
                return true;
            }
        }

        const FILE_PREFIXES: &[&str] = &[
            "/rustc/",
            "src/libstd/",
            "src/libpanic_unwind/",
            "src/libtest/",
        ];

        // Inspect filename.
        if let Some(ref filename) = self.filename {
            let filename = filename.to_string_lossy();
            if FILE_PREFIXES.iter().any(|x| filename.starts_with(x))
                || filename.contains("/.cargo/registry/src/")
            {
                return true;
            }
        }

        false
    }

    /// Heuristically determine whether a frame is likely to be a post panic
    /// frame.
    ///
    /// Post panic frames are frames of a functions called after the actual panic
    /// is already in progress and don't contain any useful information for a
    /// reader of the backtrace.
    fn is_post_panic_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "_rust_begin_unwind",
            "rust_begin_unwind",
            "core::result::unwrap_failed",
            "core::option::expect_none_failed",
            "core::panicking::panic_fmt",
            "color_backtrace::create_panic_handler",
            "std::panicking::begin_panic",
            "begin_panic_fmt",
            "failure::backtrace::Backtrace::new",
            "backtrace::capture",
            "failure::error_message::err_msg",
            "<failure::error::Error as core::convert::From<F>>::from",
        ];

        match self.name.as_ref() {
            Some(name) => SYM_PREFIXES.iter().any(|x| name.starts_with(x)),
            None => false,
        }
    }

    /// Heuristically determine whether a frame is likely to be part of language
    /// runtime.
    fn is_runtime_init_code(&self) -> bool {
        const SYM_PREFIXES: &[&str] = &[
            "std::rt::lang_start::",
            "test::run_test::run_test_inner::",
            "std::sys_common::backtrace::__rust_begin_short_backtrace",
        ];

        let (name, file) = match (self.name.as_ref(), self.filename.as_ref()) {
            (Some(name), Some(filename)) => (name, filename.to_string_lossy()),
            _ => return false,
        };

        if SYM_PREFIXES.iter().any(|x| name.starts_with(x)) {
            return true;
        }

        // For Linux, this is the best rule for skipping test init I found.
        if name == "{{closure}}" && file == "src/libtest/lib.rs" {
            return true;
        }

        false
    }
}

/// Builder for customizing the behavior of the global panic and error report hooks
pub struct HookBuilder {
    filters: Vec<Box<FilterCallback>>,
    capture_span_trace_by_default: bool,
    display_env_section: bool,
    panic_section: Option<Box<dyn Display + Send + Sync + 'static>>,
    panic_message: Box<dyn PanicMessage>,
    styles: Styles,
    #[cfg(feature = "issue-url")]
    issue_url: Option<String>,
    #[cfg(feature = "issue-url")]
    issue_metadata: Vec<(String, Box<dyn Display + Send + Sync + 'static>)>,
    #[cfg(feature = "issue-url")]
    issue_filter: Arc<IssueFilterCallback>,
}

impl HookBuilder {
    /// Construct a HookBuilder
    ///
    /// # Details
    ///
    /// By default this function calls `add_default_filters()` and
    /// `capture_span_trace_by_default(true)`. To get a `HookBuilder` with all
    /// features disabled by default call `HookBuilder::blank()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use color_eyre::config::HookBuilder;
    ///
    /// HookBuilder::new()
    ///     .install()
    ///     .unwrap();
    /// ```
    pub fn new() -> Self {
        Self::blank()
            .add_default_filters()
            .capture_span_trace_by_default(true)
    }

    /// Construct a HookBuilder with minimal features enabled
    pub fn blank() -> Self {
        HookBuilder {
            filters: vec![],
            capture_span_trace_by_default: false,
            display_env_section: true,
            panic_section: None,
            panic_message: Box::new(DefaultPanicMessage),
            styles: Styles::dark(),
            #[cfg(feature = "issue-url")]
            issue_url: None,
            #[cfg(feature = "issue-url")]
            issue_metadata: vec![],
            #[cfg(feature = "issue-url")]
            issue_filter: Arc::new(|_| true),
        }
    }

    // XXX the following tip is a bit hacky, but it's an easy way to test new styles

    /// Set the global styles that `color_eyre` should use.
    /// 
    /// Tip: An easy way to test new styles is the following:
    ///
    /// 1) clone `color_eyre` via `git clone https://github.com/yaahc/color-eyre.git`
    /// 
    /// 2) `cd color-eyre`
    ///
    /// 3) open "tests/styles.rs"
    ///
    /// 4) search for "# Easy way to test styles"
    ///
    /// 5) follow the instructions there
    ///
    /// This, basically, leverages the test code to display an error message that includes
    /// all elements that can be styled. You then can define your own styles which will
    /// be displayed.
    pub fn styles(mut self, styles: Styles) -> Self {
        self.styles = styles;
        self
    }

    /// Add a custom section to the panic hook that will be printed
    /// in the panic message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// color_eyre::config::HookBuilder::default()
    ///     .panic_section("consider reporting the bug at https://github.com/yaahc/color-eyre")
    ///     .install()
    ///     .unwrap()
    /// ```
    pub fn panic_section<S: Display + Send + Sync + 'static>(mut self, section: S) -> Self {
        self.panic_section = Some(Box::new(section));
        self
    }

    /// Overrides the main error message printing section at the start of panic
    /// reports
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::{panic::Location, fmt};
    /// use color_eyre::section::PanicMessage;
    /// use owo_colors::OwoColorize;
    ///
    /// struct MyPanicMessage;
    ///
    /// color_eyre::config::HookBuilder::default()
    ///     .panic_message(MyPanicMessage)
    ///     .install()
    ///     .unwrap();
    ///
    /// impl PanicMessage for MyPanicMessage {
    ///     fn display(&self, pi: &std::panic::PanicInfo<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         writeln!(f, "{}", "The application panicked (crashed).".red())?;
    ///
    ///         // Print panic message.
    ///         let payload = pi
    ///             .payload()
    ///             .downcast_ref::<String>()
    ///             .map(String::as_str)
    ///             .or_else(|| pi.payload().downcast_ref::<&str>().cloned())
    ///             .unwrap_or("<non string panic payload>");
    ///
    ///         write!(f, "Message:  ")?;
    ///         writeln!(f, "{}", payload.cyan())?;
    ///
    ///         // If known, print panic location.
    ///         write!(f, "Location: ")?;
    ///         if let Some(loc) = pi.location() {
    ///             write!(f, "{}", loc.file().purple())?;
    ///             write!(f, ":")?;
    ///             write!(f, "{}", loc.line().purple())?;
    ///
    ///             write!(f, "\n\nConsider reporting the bug at {}", custom_url(loc, payload))?;
    ///         } else {
    ///             write!(f, "<unknown>")?;
    ///         }
    ///
    ///         Ok(())
    ///     }
    /// }
    ///
    /// fn custom_url(location: &Location<'_>, message: &str) -> impl fmt::Display {
    ///     "todo"
    /// }
    /// ```
    pub fn panic_message<S: PanicMessage>(mut self, section: S) -> Self {
        self.panic_message = Box::new(section);
        self
    }

    /// Set an upstream github repo and enable issue reporting url generation
    ///
    /// # Details
    ///
    /// Once enabled, color-eyre will generate urls that will create customized
    /// issues pre-populated with information about the associated error report.
    ///
    /// Additional information can be added to the metadata table in the
    /// generated urls by calling `add_issue_metadata` when configuring the
    /// HookBuilder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// color_eyre::config::HookBuilder::default()
    ///     .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
    ///     .install()
    ///     .unwrap();
    /// ```
    #[cfg(feature = "issue-url")]
    #[cfg_attr(docsrs, doc(cfg(feature = "issue-url")))]
    pub fn issue_url<S: ToString>(mut self, url: S) -> Self {
        self.issue_url = Some(url.to_string());
        self
    }

    /// Add a new entry to the metadata table in generated github issue urls
    ///
    /// **Note**: this metadata will be ignored if no `issue_url` is set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// color_eyre::config::HookBuilder::default()
    ///     .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
    ///     .add_issue_metadata("version", env!("CARGO_PKG_VERSION"))
    ///     .install()
    ///     .unwrap();
    /// ```
    #[cfg(feature = "issue-url")]
    #[cfg_attr(docsrs, doc(cfg(feature = "issue-url")))]
    pub fn add_issue_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Display,
        V: Display + Send + Sync + 'static,
    {
        let pair = (key.to_string(), Box::new(value) as _);
        self.issue_metadata.push(pair);
        self
    }

    /// Configures a filter for disabling issue url generation for certain kinds of errors
    ///
    /// If the closure returns `true`, then the issue url will be generated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// color_eyre::config::HookBuilder::default()
    ///     .issue_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new"))
    ///     .issue_filter(|kind| match kind {
    ///         color_eyre::ErrorKind::NonRecoverable(payload) => {
    ///             let payload = payload
    ///                 .downcast_ref::<String>()
    ///                 .map(String::as_str)
    ///                 .or_else(|| payload.downcast_ref::<&str>().cloned())
    ///                 .unwrap_or("<non string panic payload>");
    ///
    ///             !payload.contains("my irrelevant error message")
    ///         },
    ///         color_eyre::ErrorKind::Recoverable(error) => !error.is::<std::fmt::Error>(),
    ///     })
    ///     .install()
    ///     .unwrap();
    ///
    #[cfg(feature = "issue-url")]
    #[cfg_attr(docsrs, doc(cfg(feature = "issue-url")))]
    pub fn issue_filter<F>(mut self, predicate: F) -> Self
    where
        F: Fn(crate::ErrorKind<'_>) -> bool + Send + Sync + 'static,
    {
        self.issue_filter = Arc::new(predicate);
        self
    }

    /// Configures the default capture mode for `SpanTraces` in error reports and panics
    pub fn capture_span_trace_by_default(mut self, cond: bool) -> Self {
        self.capture_span_trace_by_default = cond;
        self
    }

    /// Configures the enviroment varible info section and whether or not it is displayed
    pub fn display_env_section(mut self, cond: bool) -> Self {
        self.display_env_section = cond;
        self
    }

    /// Add a custom filter to the set of frame filters
    ///
    /// # Examples
    ///
    /// ```rust
    /// color_eyre::config::HookBuilder::default()
    ///     .add_frame_filter(Box::new(|frames| {
    ///         let filters = &[
    ///             "uninteresting_function",
    ///         ];
    ///
    ///         frames.retain(|frame| {
    ///             !filters.iter().any(|f| {
    ///                 let name = if let Some(name) = frame.name.as_ref() {
    ///                     name.as_str()
    ///                 } else {
    ///                     return true;
    ///                 };
    ///
    ///                 name.starts_with(f)
    ///             })
    ///         });
    ///     }))
    ///     .install()
    ///     .unwrap();
    /// ```
    pub fn add_frame_filter(mut self, filter: Box<FilterCallback>) -> Self {
        self.filters.push(filter);
        self
    }

    /// Install the given Hook as the global error report hook
    pub fn install(self) -> Result<(), crate::eyre::Report> {

        // XXX I'm not 100% sure if the following is the ideal way to handle these failures (however, I assume it is).

        if STYLES.set(self.styles).is_err() {
            // XXX this error can only happen if `install` is called more than one time (because `STYLES` is private)
            Err(InstallStylesError)?
        }

        if color_spantrace::set_styles(self.styles.into()).is_err() {
            // XXX this error can happen if `install` is called more than one time, or if the user already called `color_spantrace::set_styles` or `color_spantrace::colorize`
            Err(InstallColorBacktraceStylesError)?
        }

        let (panic_hook, eyre_hook) = self.into_hooks();
        crate::eyre::set_hook(Box::new(move |e| Box::new(eyre_hook.default(e))))?;
        install_panic_hook();

        if crate::CONFIG.set(panic_hook).is_err() {
            Err(InstallError)?
        }

        Ok(())
    }

    /// Add the default set of filters to this `HookBuilder`'s configuration
    pub fn add_default_filters(self) -> Self {
        self.add_frame_filter(Box::new(default_frame_filter))
            .add_frame_filter(Box::new(eyre_frame_filters))
    }

    pub(crate) fn into_hooks(self) -> (PanicHook, EyreHook) {
        #[cfg(feature = "issue-url")]
        let metadata = Arc::new(self.issue_metadata);
        let panic_hook = PanicHook {
            filters: self.filters.into_iter().map(Into::into).collect(),
            section: self.panic_section,
            #[cfg(feature = "capture-spantrace")]
            capture_span_trace_by_default: self.capture_span_trace_by_default,
            display_env_section: self.display_env_section,
            panic_message: self.panic_message,
            styles: self.styles,
            #[cfg(feature = "issue-url")]
            issue_url: self.issue_url.clone(),
            #[cfg(feature = "issue-url")]
            issue_metadata: metadata.clone(),
            #[cfg(feature = "issue-url")]
            issue_filter: self.issue_filter.clone(),
        };

        let eyre_hook = EyreHook {
            #[cfg(feature = "capture-spantrace")]
            capture_span_trace_by_default: self.capture_span_trace_by_default,
            display_env_section: self.display_env_section,
            styles: self.styles,
            #[cfg(feature = "issue-url")]
            issue_url: self.issue_url,
            #[cfg(feature = "issue-url")]
            issue_metadata: metadata,
            #[cfg(feature = "issue-url")]
            issue_filter: self.issue_filter,
        };

        (panic_hook, eyre_hook)
    }
}

impl From<Styles> for color_spantrace::Styles {
    fn from(src: Styles) -> color_spantrace::Styles {
        color_spantrace::Styles::new()
            .file(src.file)
            .line_number(src.line_number)
            .target(src.spantrace_target)
            .fields(src.spantrace_fields)
            .active_line(src.active_line)
    }
}

#[allow(missing_docs)]
impl Default for HookBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn default_frame_filter(frames: &mut Vec<&Frame>) {
    let top_cutoff = frames
        .iter()
        .rposition(|x| x.is_post_panic_code())
        .map(|x| x + 2) // indices are 1 based
        .unwrap_or(0);

    let bottom_cutoff = frames
        .iter()
        .position(|x| x.is_runtime_init_code())
        .unwrap_or_else(|| frames.len());

    let rng = top_cutoff..=bottom_cutoff;
    frames.retain(|x| rng.contains(&x.n))
}

fn eyre_frame_filters(frames: &mut Vec<&Frame>) {
    let filters = &[
        "<color_eyre::Handler as eyre::EyreHandler>::default",
        "eyre::",
        "color_eyre::",
    ];

    frames.retain(|frame| {
        !filters.iter().any(|f| {
            let name = if let Some(name) = frame.name.as_ref() {
                name.as_str()
            } else {
                return true;
            };

            name.starts_with(f)
        })
    });
}

struct PanicPrinter<'a>(&'a std::panic::PanicInfo<'a>);

impl fmt::Display for PanicPrinter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print_panic_info(self, f)
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|pi| eprintln!("{}", PanicPrinter(pi))))
}

struct DefaultPanicMessage;

impl PanicMessage for DefaultPanicMessage {
    fn display(&self, pi: &std::panic::PanicInfo<'_>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // XXX is my assumption correct that this function is guaranteed to only run after `color_eyre` was setup successfully (including setting `STYLES`), and that therefore the following line will never panic? Otherwise, we could return `fmt::Error`, but if the above is true, I like `unwrap` + a comment why this never fails better
        let styles = crate::config::STYLES.get().unwrap();

        writeln!(f, "{}", "The application panicked (crashed).".style(styles.panic_header))?;

        // Print panic message.
        let payload = pi
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| pi.payload().downcast_ref::<&str>().cloned())
            .unwrap_or("<non string panic payload>");

        write!(f, "Message:  ")?;
        writeln!(f, "{}", payload.style(styles.panic_message))?;

        // If known, print panic location.
        write!(f, "Location: ")?;
        if let Some(loc) = pi.location() {
            write!(f, "{}", loc.file().style(styles.panic_file))?;
            write!(f, ":")?;
            write!(f, "{}", loc.line().style(styles.panic_line_number))?;
        } else {
            write!(f, "<unknown>")?;
        }

        Ok(())
    }
}

fn print_panic_info(printer: &PanicPrinter<'_>, out: &mut fmt::Formatter<'_>) -> fmt::Result {
    let hook = installed_hook();
    hook.panic_message.display(printer.0, out)?;

    let v = panic_verbosity();
    let capture_bt = v != Verbosity::Minimal;

    #[cfg(feature = "capture-spantrace")]
    let span_trace = if hook.spantrace_capture_enabled() {
        Some(tracing_error::SpanTrace::capture())
    } else {
        None
    };

    let bt = if capture_bt {
        Some(backtrace::Backtrace::new())
    } else {
        None
    };

    let mut separated = out.header("\n\n");

    if let Some(ref section) = hook.section {
        write!(&mut separated.ready(), "{}", section)?;
    }

    #[cfg(feature = "capture-spantrace")]
    {
        if let Some(span_trace) = span_trace.as_ref() {
            write!(
                &mut separated.ready(),
                "{}",
                crate::writers::FormattedSpanTrace(span_trace)
            )?;
        }
    }

    if let Some(bt) = bt.as_ref() {
        let fmted_bt = hook.format_backtrace(&bt);
        write!(
            indented(&mut separated.ready()).with_format(Format::Uniform { indentation: "  " }),
            "{}",
            fmted_bt
        )?;
    }

    if hook.display_env_section {
        let env_section = EnvSection {
            bt_captured: &capture_bt,
            #[cfg(feature = "capture-spantrace")]
            span_trace: span_trace.as_ref(),
        };

        write!(&mut separated.ready(), "{}", env_section)?;
    }

    #[cfg(feature = "issue-url")]
    {
        let payload = printer.0.payload();

        if hook.issue_url.is_some()
            && (*hook.issue_filter)(crate::ErrorKind::NonRecoverable(payload))
        {
            let url = hook.issue_url.as_ref().unwrap();
            let payload = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().cloned())
                .unwrap_or("<non string panic payload>");

            let issue_section = crate::section::github::IssueSection::new(url, payload)
                .with_backtrace(bt.as_ref())
                .with_location(printer.0.location())
                .with_metadata(&**hook.issue_metadata);

            #[cfg(feature = "capture-spantrace")]
            let issue_section = issue_section.with_span_trace(span_trace.as_ref());

            write!(&mut separated.ready(), "{}", issue_section)?;
        }
    }

    Ok(())
}

pub(crate) struct PanicHook {
    filters: Vec<Arc<FilterCallback>>,
    section: Option<Box<dyn Display + Send + Sync + 'static>>,
    panic_message: Box<dyn PanicMessage>,
    styles: Styles,
    #[cfg(feature = "capture-spantrace")]
    capture_span_trace_by_default: bool,
    display_env_section: bool,
    #[cfg(feature = "issue-url")]
    issue_url: Option<String>,
    #[cfg(feature = "issue-url")]
    issue_metadata: Arc<Vec<(String, Box<dyn Display + Send + Sync + 'static>)>>,
    #[cfg(feature = "issue-url")]
    issue_filter: Arc<IssueFilterCallback>,
}

impl PanicHook {
    pub(crate) fn format_backtrace<'a>(
        &'a self,
        trace: &'a backtrace::Backtrace,
    ) -> BacktraceFormatter<'a> {
        BacktraceFormatter {
            printer: self,
            inner: trace,
            styles: self.styles,
        }
    }

    #[cfg(feature = "capture-spantrace")]
    fn spantrace_capture_enabled(&self) -> bool {
        std::env::var("RUST_SPANTRACE")
            .map(|val| val != "0")
            .unwrap_or(self.capture_span_trace_by_default)
    }
}

pub(crate) struct EyreHook {
    #[cfg(feature = "capture-spantrace")]
    capture_span_trace_by_default: bool,
    display_env_section: bool,
    styles: Styles,
    #[cfg(feature = "issue-url")]
    issue_url: Option<String>,
    #[cfg(feature = "issue-url")]
    issue_metadata: Arc<Vec<(String, Box<dyn Display + Send + Sync + 'static>)>>,
    #[cfg(feature = "issue-url")]
    issue_filter: Arc<IssueFilterCallback>,
}

impl EyreHook {
    #[allow(unused_variables)]
    pub(crate) fn default(&self, error: &(dyn std::error::Error + 'static)) -> crate::Handler {
        let backtrace = if lib_verbosity() != Verbosity::Minimal {
            Some(backtrace::Backtrace::new())
        } else {
            None
        };

        #[cfg(feature = "capture-spantrace")]
        let span_trace = if self.spantrace_capture_enabled()
            && crate::handler::get_deepest_spantrace(error).is_none()
        {
            Some(tracing_error::SpanTrace::capture())
        } else {
            None
        };

        crate::Handler {
            backtrace,
            #[cfg(feature = "capture-spantrace")]
            span_trace,
            sections: Vec::new(),
            display_env_section: self.display_env_section,
            #[cfg(feature = "issue-url")]
            issue_url: self.issue_url.clone(),
            #[cfg(feature = "issue-url")]
            issue_metadata: self.issue_metadata.clone(),
            #[cfg(feature = "issue-url")]
            issue_filter: self.issue_filter.clone(),
            styles: self.styles,
        }
    }

    #[cfg(feature = "capture-spantrace")]
    fn spantrace_capture_enabled(&self) -> bool {
        std::env::var("RUST_SPANTRACE")
            .map(|val| val != "0")
            .unwrap_or(self.capture_span_trace_by_default)
    }
}

pub(crate) struct BacktraceFormatter<'a> {
    printer: &'a PanicHook,
    inner: &'a backtrace::Backtrace,
    styles: Styles,
}

impl fmt::Display for BacktraceFormatter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:━^80}", " BACKTRACE ")?;

        // Collect frame info.
        let frames: Vec<_> = self
            .inner
            .frames()
            .iter()
            .flat_map(|frame| frame.symbols())
            .zip(1usize..)
            .map(|(sym, n)| Frame {
                name: sym.name().map(|x| x.to_string()),
                lineno: sym.lineno(),
                filename: sym.filename().map(|x| x.into()),
                n,
                styles: self.styles,
            })
            .collect();

        let mut filtered_frames = frames.iter().collect();
        match env::var("COLORBT_SHOW_HIDDEN").ok().as_deref() {
            Some("1") | Some("on") | Some("y") => (),
            _ => {
                for filter in &self.printer.filters {
                    filter(&mut filtered_frames);
                }
            }
        }

        if filtered_frames.is_empty() {
            // TODO: Would probably look better centered.
            return write!(f, "\n<empty backtrace>");
        }

        let mut separated = f.header("\n");

        // Don't let filters mess with the order.
        filtered_frames.sort_by_key(|x| x.n);

        let mut buf = String::new();

        macro_rules! print_hidden {
            ($n:expr) => {
                let n = $n;
                buf.clear();
                write!(
                    &mut buf,
                    "{decorator} {n} frame{plural} hidden {decorator}",
                    n = n,
                    plural = if n == 1 { "" } else { "s" },
                    decorator = "⋮",
                )
                .expect("writing to strings doesn't panic");
                write!(&mut separated.ready(), "{:^80}", buf.style(self.styles.hidden_frames))?;
            };
        }

        let mut last_n = 0;
        for frame in &filtered_frames {
            let frame_delta = frame.n - last_n - 1;
            if frame_delta != 0 {
                print_hidden!(frame_delta);
            }
            write!(&mut separated.ready(), "{}", frame)?;
            last_n = frame.n;
        }

        let last_filtered_n = filtered_frames.last().unwrap().n;
        let last_unfiltered_n = frames.last().unwrap().n;
        if last_filtered_n < last_unfiltered_n {
            print_hidden!(last_unfiltered_n - last_filtered_n);
        }

        Ok(())
    }
}

pub(crate) fn installed_hook() -> &'static PanicHook {
    crate::CONFIG.get_or_init(default_printer)
}

fn default_printer() -> PanicHook {
    let (panic_hook, _) = HookBuilder::default().into_hooks();
    panic_hook
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub(crate) enum Verbosity {
    Minimal,
    Medium,
    Full,
}

pub(crate) fn panic_verbosity() -> Verbosity {
    match env::var("RUST_BACKTRACE") {
        Ok(s) if s == "full" => Verbosity::Full,
        Ok(s) if s != "0" => Verbosity::Medium,
        _ => Verbosity::Minimal,
    }
}

pub(crate) fn lib_verbosity() -> Verbosity {
    match env::var("RUST_LIB_BACKTRACE").or_else(|_| env::var("RUST_BACKTRACE")) {
        Ok(s) if s == "full" => Verbosity::Full,
        Ok(s) if s != "0" => Verbosity::Medium,
        _ => Verbosity::Minimal,
    }
}

/// Callback for filtering a vector of `Frame`s
pub type FilterCallback = dyn Fn(&mut Vec<&Frame>) + Send + Sync + 'static;

/// Callback for filtering issue url generation in error reports
#[cfg(feature = "issue-url")]
#[cfg_attr(docsrs, doc(cfg(feature = "issue-url")))]
pub type IssueFilterCallback = dyn Fn(crate::ErrorKind<'_>) -> bool + Send + Sync + 'static;
