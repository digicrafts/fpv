use std::path::Path;
use tree_sitter::Language;
use tree_sitter_highlight::HighlightConfiguration;

const EMPTY_QUERY: &str = "";
pub const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "comment",
    "constant",
    "constant.builtin",
    "constructor",
    "embedded",
    "escape",
    "function",
    "function.builtin",
    "function.method",
    "keyword",
    "module",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "tag",
    "text",
    "text.emphasis",
    "text.literal",
    "text.reference",
    "text.strong",
    "text.title",
    "text.uri",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "none",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighlightLanguage {
    Bash,
    C,
    Cpp,
    Css,
    Go,
    Html,
    Java,
    JavaScript,
    Jsx,
    Json,
    Markdown,
    Python,
    Rust,
    Toml,
    TypeScript,
    Tsx,
}

struct LanguageSpec {
    language_id: &'static str,
    language: Language,
    highlights_query: String,
    injections_query: &'static str,
    locals_query: &'static str,
}

pub struct HighlightTarget<'a> {
    pub language_id: &'static str,
    pub config: &'a HighlightConfiguration,
}

pub struct HighlightContext {
    bash_config: Option<HighlightConfiguration>,
    c_config: Option<HighlightConfiguration>,
    cpp_config: Option<HighlightConfiguration>,
    css_config: Option<HighlightConfiguration>,
    go_config: Option<HighlightConfiguration>,
    html_config: Option<HighlightConfiguration>,
    java_config: Option<HighlightConfiguration>,
    js_config: Option<HighlightConfiguration>,
    jsx_config: Option<HighlightConfiguration>,
    json_config: Option<HighlightConfiguration>,
    markdown_config: Option<HighlightConfiguration>,
    markdown_inline_config: Option<HighlightConfiguration>,
    py_config: Option<HighlightConfiguration>,
    rs_config: Option<HighlightConfiguration>,
    toml_config: Option<HighlightConfiguration>,
    ts_config: Option<HighlightConfiguration>,
    tsx_config: Option<HighlightConfiguration>,
}

impl HighlightLanguage {
    fn language_id(self) -> &'static str {
        match self {
            Self::Bash => "Shell",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Css => "CSS",
            Self::Go => "Go",
            Self::Html => "HTML",
            Self::Java => "Java",
            Self::JavaScript | Self::Jsx => "JavaScript",
            Self::Json => "JSON",
            Self::Markdown => "Markdown",
            Self::Python => "Python",
            Self::Rust => "Rust",
            Self::Toml => "TOML",
            Self::TypeScript | Self::Tsx => "TypeScript",
        }
    }

    fn spec(self) -> LanguageSpec {
        match self {
            Self::Bash => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_bash::language(),
                highlights_query: tree_sitter_bash::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::C => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_c::language(),
                highlights_query: tree_sitter_c::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Cpp => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_cpp::language(),
                highlights_query: tree_sitter_cpp::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Css => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_css::language(),
                highlights_query: tree_sitter_css::HIGHLIGHTS_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Go => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_go::language(),
                highlights_query: tree_sitter_go::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Html => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_html::language(),
                highlights_query: tree_sitter_html::HIGHLIGHTS_QUERY.to_string(),
                injections_query: tree_sitter_html::INJECTIONS_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Java => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_java::language(),
                highlights_query: tree_sitter_java::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::JavaScript => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_javascript::language(),
                highlights_query: tree_sitter_javascript::HIGHLIGHT_QUERY.to_string(),
                injections_query: tree_sitter_javascript::INJECTION_QUERY,
                locals_query: tree_sitter_javascript::LOCALS_QUERY,
            },
            Self::Jsx => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_javascript::language(),
                highlights_query: format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY
                ),
                injections_query: tree_sitter_javascript::INJECTION_QUERY,
                locals_query: tree_sitter_javascript::LOCALS_QUERY,
            },
            Self::Json => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_json::language(),
                highlights_query: tree_sitter_json::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Markdown => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_md::language(),
                highlights_query: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.to_string(),
                injections_query: tree_sitter_md::INJECTION_QUERY_BLOCK,
                locals_query: EMPTY_QUERY,
            },
            Self::Python => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_python::language(),
                highlights_query: tree_sitter_python::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Rust => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_rust::language(),
                highlights_query: tree_sitter_rust::HIGHLIGHT_QUERY.to_string(),
                injections_query: tree_sitter_rust::INJECTIONS_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::Toml => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_toml::language(),
                highlights_query: tree_sitter_toml::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: EMPTY_QUERY,
            },
            Self::TypeScript => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_typescript::language_typescript(),
                highlights_query: tree_sitter_typescript::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: tree_sitter_typescript::LOCALS_QUERY,
            },
            Self::Tsx => LanguageSpec {
                language_id: self.language_id(),
                language: tree_sitter_typescript::language_tsx(),
                highlights_query: tree_sitter_typescript::HIGHLIGHT_QUERY.to_string(),
                injections_query: EMPTY_QUERY,
                locals_query: tree_sitter_typescript::LOCALS_QUERY,
            },
        }
    }

    fn for_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
        match ext.as_str() {
            "bash" | "sh" | "zsh" | "ksh" => Some(Self::Bash),
            "c" | "h" => Some(Self::C),
            "cc" | "cp" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => Some(Self::Cpp),
            "css" => Some(Self::Css),
            "go" => Some(Self::Go),
            "html" | "htm" => Some(Self::Html),
            "java" => Some(Self::Java),
            "js" | "mjs" | "cjs" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "json" => Some(Self::Json),
            "md" | "markdown" => Some(Self::Markdown),
            "py" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            _ => None,
        }
    }
}

fn build_config(language: HighlightLanguage) -> Option<HighlightConfiguration> {
    let spec = language.spec();
    let mut config = HighlightConfiguration::new(
        spec.language,
        &spec.highlights_query,
        spec.injections_query,
        spec.locals_query,
    )
    .ok()?;
    config.configure(HIGHLIGHT_NAMES);
    Some(config)
}

fn build_markdown_inline_config() -> Option<HighlightConfiguration> {
    let mut config = HighlightConfiguration::new(
        tree_sitter_md::inline_language(),
        tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
        tree_sitter_md::INJECTION_QUERY_INLINE,
        EMPTY_QUERY,
    )
    .ok()?;
    config.configure(HIGHLIGHT_NAMES);
    Some(config)
}

impl HighlightContext {
    pub fn new() -> Self {
        Self {
            bash_config: build_config(HighlightLanguage::Bash),
            c_config: build_config(HighlightLanguage::C),
            cpp_config: build_config(HighlightLanguage::Cpp),
            css_config: build_config(HighlightLanguage::Css),
            go_config: build_config(HighlightLanguage::Go),
            html_config: build_config(HighlightLanguage::Html),
            java_config: build_config(HighlightLanguage::Java),
            js_config: build_config(HighlightLanguage::JavaScript),
            jsx_config: build_config(HighlightLanguage::Jsx),
            json_config: build_config(HighlightLanguage::Json),
            markdown_config: build_config(HighlightLanguage::Markdown),
            markdown_inline_config: build_markdown_inline_config(),
            py_config: build_config(HighlightLanguage::Python),
            rs_config: build_config(HighlightLanguage::Rust),
            toml_config: build_config(HighlightLanguage::Toml),
            ts_config: build_config(HighlightLanguage::TypeScript),
            tsx_config: build_config(HighlightLanguage::Tsx),
        }
    }

    pub fn target_for_path(&self, path: &Path) -> Option<HighlightTarget<'_>> {
        let language = HighlightLanguage::for_path(path)?;
        let spec = language.spec();
        let config = match language {
            HighlightLanguage::Bash => self.bash_config.as_ref()?,
            HighlightLanguage::C => self.c_config.as_ref()?,
            HighlightLanguage::Cpp => self.cpp_config.as_ref()?,
            HighlightLanguage::Css => self.css_config.as_ref()?,
            HighlightLanguage::Go => self.go_config.as_ref()?,
            HighlightLanguage::Html => self.html_config.as_ref()?,
            HighlightLanguage::Java => self.java_config.as_ref()?,
            HighlightLanguage::JavaScript => self.js_config.as_ref()?,
            HighlightLanguage::Jsx => self.jsx_config.as_ref()?,
            HighlightLanguage::Json => self.json_config.as_ref()?,
            HighlightLanguage::Markdown => self.markdown_config.as_ref()?,
            HighlightLanguage::Python => self.py_config.as_ref()?,
            HighlightLanguage::Rust => self.rs_config.as_ref()?,
            HighlightLanguage::Toml => self.toml_config.as_ref()?,
            HighlightLanguage::TypeScript => self.ts_config.as_ref()?,
            HighlightLanguage::Tsx => self.tsx_config.as_ref()?,
        };
        Some(HighlightTarget {
            language_id: spec.language_id,
            config,
        })
    }

    pub fn injection_config(&self, language_name: &str) -> Option<&HighlightConfiguration> {
        match language_name.to_ascii_lowercase().as_str() {
            "markdown_inline" => self.markdown_inline_config.as_ref(),
            "bash" | "sh" | "shell" | "zsh" | "ksh" => self.bash_config.as_ref(),
            "c" => self.c_config.as_ref(),
            "cc" | "cp" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => {
                self.cpp_config.as_ref()
            }
            "css" => self.css_config.as_ref(),
            "go" | "golang" => self.go_config.as_ref(),
            "html" | "htm" => self.html_config.as_ref(),
            "java" => self.java_config.as_ref(),
            "javascript" | "js" | "mjs" | "cjs" => self.js_config.as_ref(),
            "jsx" => self.jsx_config.as_ref(),
            "json" => self.json_config.as_ref(),
            "python" | "py" => self.py_config.as_ref(),
            "rust" | "rs" => self.rs_config.as_ref(),
            "toml" => self.toml_config.as_ref(),
            "typescript" | "ts" => self.ts_config.as_ref(),
            "tsx" => self.tsx_config.as_ref(),
            _ => None,
        }
    }
}

impl Default for HighlightContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn language_for_path(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    let lang = match ext.as_str() {
        "bash" | "sh" | "zsh" | "ksh" => "Shell",
        "c" | "h" => "C",
        "cc" | "cp" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => "C++",
        "css" => "CSS",
        "go" => "Go",
        "html" | "htm" => "HTML",
        "java" => "Java",
        "md" | "markdown" => "Markdown",
        "py" => "Python",
        "json" => "JSON",
        "rs" => "Rust",
        "toml" => "TOML",
        "yaml" | "yml" => "YAML",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "tsx" | "ts" => "TypeScript",
        "xml" => "XML",
        _ => return None,
    };
    Some(lang.to_string())
}
