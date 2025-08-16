# prmt 🚀

Ultra-fast, customizable shell prompt generator written in Rust. Features zero-copy parsing with SIMD optimizations for sub-microsecond template processing.

## Features

- **⚡ Blazing Fast**: Sub-microsecond parsing with memchr SIMD optimizations
- **🔧 Modular Architecture**: Clean separation with Module trait and registry system
- **🎨 Rich Template Language**: 5-field format with styles, formats, and affixes
- **📦 Zero-Copy Parsing**: 50-70% reduction in allocations
- **🦀 Memory Efficient**: Single-pass parser with lazy unescaping
- **🚀 Parallel Detection**: Module detection runs concurrently via Rayon
- **🔥 Gitoxide Integration**: 2.9x faster git operations
- **✨ Smart Rendering**: Conditional display based on context

## Performance

### Parser Performance (Template Processing)
| Template Type | Performance | Improvement |
|--------------|-------------|-------------|
| Simple `{path}` | ~0.2 µs | 3.7x faster |
| Complex (5 modules + styles) | ~1.1 µs | 4.7x faster |
| Long text (100+ chars) | ~0.1 µs | 7.6x faster |
| Many placeholders | ~2.0 µs | 3.8x faster |

### End-to-End Performance
| Scenario | Time | Notes |
|----------|------|-------|
| Path only | ~0.01ms | Minimal prompt |
| Path + Git | ~1-2ms | Branch and status |
| With Rust version | ~25-30ms | Includes `rustc --version` |
| With `--no-version` | <5ms | Skips all version detection |

### Key Optimizations

**Parser (NEW)**:
- **Zero-copy parsing** - Text sections are slices, not allocations
- **SIMD scanning** - memchr finds `{`, `}`, `\` in parallel
- **Lazy unescaping** - Only allocates for fields with backslashes
- **Single-pass** - No backtracking or re-parsing

**Runtime**:
- **Gitoxide (gix)** - 2.9x faster than git2 for git operations
- **Parallel detection** - All modules detected simultaneously via Rayon
- **Direct execution** - No compilation or caching overhead
- **Conditional rendering** - Modules only render when conditions met

## Installation

```bash
# Install from crates.io
cargo install prmt

# Build from source (Rust 2024 edition required)
cargo build --release
cp target/release/prmt ~/.local/bin/

# Or install directly
cargo install --path .

# Verify installation
prmt --version
```

## Format Specification

### Format Syntax
```
{module}                      - Default everything
{module:style}                - Custom style
{module:style:type}           - Custom style and type
{module:style:type:prefix}    - Add prefix to value
{module:style:type:prefix:postfix} - Add prefix and postfix

# Omitting parts (empty means default)
{module::::suffix}            - Default style/type, suffix only
{module:::prefix:}            - Default style/type, prefix only
{module:::prefix:suffix}      - Default style/type, both prefix/suffix
{module::type}                - No style, specific type
{module::type::suffix}        - No style, specific type, suffix only
```

### Available Modules

| Module | Detection | Description |
|--------|-----------|-------------|
| `path` | Always active | Current directory with ~ for home |
| `ok` | Exit code = 0 | Shows when last command succeeded (default: ❯) |
| `fail` | Exit code ≠ 0 | Shows when last command failed (default: ❯) |
| `git` | `.git` directory | Branch name with status indicators |
| `node` | `package.json` | Node.js version |
| `python` | `requirements.txt`, `pyproject.toml`, etc | Python version |
| `rust` | `Cargo.toml` | Rust version |
| `deno` | `deno.json`, `deno.jsonc` | Deno version |
| `bun` | `bun.lockb` | Bun version |
| `go` | `go.mod` | Go version |

### Type Values

**Version modules** (rust, node, python, etc.):
- `full` or `f` - Full version (1.89.0)
- `short` or `s` - Major.minor (1.89)
- `major` or `m` - Major only (1)

**Path module**:
- `relative` or `r` - Path with ~ for home directory (default)
- `absolute` or `a` - Full absolute path without ~ substitution
- `short` or `s` - Last directory only

**Git module**:
- `full` or `f` - Branch with status (default)
- `short` or `s` - Branch name only

**Ok/Fail modules**:
- `full` - Default symbol (❯)
- `code` - Shows the actual exit code number
- *Any other string* - Uses that string as the symbol (e.g., `{ok::✓}` shows ✓)

### Type Validation

The format parser validates types at parse time to catch errors early:

```bash
# Valid types for each module
prmt '{path::short}'     # ✓ Valid
prmt '{rust::major}'     # ✓ Valid  
prmt '{ok::✓}'          # ✓ Valid (custom symbol)
prmt '{fail::code}'     # ✓ Valid (shows exit code)

# Invalid types produce clear errors
prmt '{path::major}'
# Error: Invalid type 'major' for module 'path'. Valid types: relative, r, absolute, a, short, s

prmt '{git::major}'
# Error: Invalid type 'major' for module 'git'. Valid types: full, short
```

### Default Module Styles

| Module | Default Color | Can Override |
|--------|--------------|--------------|
| `path` | cyan | Yes |
| `ok` | green | Yes |
| `fail` | red | Yes |
| `git` | purple | Yes |
| `node` | green | Yes |
| `rust` | red | Yes |
| `python` | yellow | Yes |
| `go` | cyan | Yes |
| `deno` | - | Yes |
| `bun` | - | Yes |

### Styles

**Colors**: `black`, `red`, `green`, `yellow`, `blue`, `purple`, `cyan`, `white`, `#hexcode`

**Modifiers**: `bold`, `dim`, `italic`, `underline`, `reverse`, `strikethrough`

Combine with dots: `cyan.bold`, `red.dim.italic`

### Escaping

- `\{` → `{` (literal brace)
- `\}` → `}` (literal brace)
- `\n` → newline
- `\t` → tab
- `\:` → `:` (literal colon in fields)
- `\\` → `\` (literal backslash)

## Usage Examples

```bash
# Simple format with defaults
prmt '{path} {rust} {git}'
# Output: ~/projects 1.89.0 master

# Format with types and styles
prmt '{path::a}'                  # /home/user/projects (absolute path)
prmt '{path::r}'                  # ~/projects (relative with ~)
prmt '{path::s}'                  # projects (short - last dir only)
prmt '{rust:red:s}'               # 1.89 in red (short version)
prmt '{rust:red:m:v:}'            # v1 in red (major version with prefix)
prmt '{path:cyan:s:[:]}'          # [projects] in cyan
prmt '{git:purple::on :}'         # on master in purple

# Simplified formats with omitted parts
prmt '{rust::::!}'                # 1.89.0! (default style/type, suffix only)
prmt '{rust:::v:}'                # v1.89.0 (default style/type, prefix only)
prmt '{path::::]}'                # ~/projects] (suffix only)
prmt '{git:::on :}'               # on master (prefix only)

# Add your own icons with prefix
prmt '{rust:::🦀 :}'              # 🦀 1.89.0 (default color)
prmt '{node:green::⬢ :}'          # ⬢ 20.5.0 in green
prmt '{python:yellow::🐍 :}'      # 🐍 3.11.0 in yellow

# Using short format aliases
prmt '{path:cyan:s} {rust:red:m:v:}' # projects v1 (both in color)
prmt '{git::s:on :}'              # on master (short format with prefix)

# No style with type  
prmt '{path::s}'                  # projects (no color, short)
prmt '{path::a}'                  # /home/user/projects (no color, absolute)
prmt '{rust::m:v}'                # v1 (no color, major with prefix)

# With exit code indicators (requires --code flag)
prmt --code $? '{path:cyan} {ok:green}{fail:red}'
# Output (success): ~/projects ❯ (green)
# Output (failure): ~/projects ❯ (red)

# Fast mode (no version detection)
prmt --no-version '{path:cyan} {rust:red} {node:green}'
# Output: ~/projects (only shows active modules, no versions)

# Custom symbols for ok/fail using type as symbol
prmt --code $? '{path} {ok::✓} {fail::✗}'
# Output (success): ~/projects ✓
# Output (failure): ~/projects ✗

# Show exit code on failure
prmt --code $? '{path} {ok::❯} {fail::code}'
# Output (success): ~/projects ❯
# Output (failure with code 127): ~/projects 127
```

## Shell Integration

### Bash
```bash
# Add to ~/.bashrc
PS1='$(prmt --code $? "{path:cyan:s} {git:purple:s:on :} {ok:green}{fail:red}")\$ '

# Or set via environment variable
export PRMT_FORMAT="{path:cyan:r} {rust:red:m:🦀 v:} {git:purple}"
PS1='$(prmt --code $?)\$ '
```

### Zsh
```zsh
# Add to ~/.zshrc
PROMPT='$(prmt --code $? "{path:cyan:s} {git:purple:s:on :} {ok:green}{fail:red}") '

# Or with environment variable
export PRMT_FORMAT="{path:cyan:r} {node:green:s:⬢ :} {git:purple}"
PROMPT='$(prmt --code $?) '
```

### Fish
```fish
# Add to ~/.config/fish/config.fish
function fish_prompt
    prmt --code $status '{path:cyan:s} {git:purple:s:on :} {ok:green}{fail:red} '
end

# Or with environment variable
set -x PRMT_FORMAT "{path:cyan:r} {python:yellow:m:🐍 :} {git:purple}"
function fish_prompt
    prmt --code $status
end
```

### PowerShell
```powershell
# Add to $PROFILE
function prompt {
    prmt --code $LASTEXITCODE '{path:cyan:s} {git:purple:s:on :} {ok:green}{fail:red} '
}

# Or with environment variable
$env:PRMT_FORMAT = "{path:cyan:r} {git:purple}"
function prompt {
    prmt --code $LASTEXITCODE
}
```

## Command-Line Options

```
prmt [OPTIONS] [FORMAT]

OPTIONS:
    -n, --no-version    Skip version detection for speed
    -d, --debug         Show debug information and timing
    -b, --bench         Run benchmark (100 iterations)
        --code <CODE>   Exit code of the last command (for ok/fail modules)
    -h, --help         Print help
    -V, --version      Print version

ARGS:
    <FORMAT>           Format string (default from PRMT_FORMAT env var)
```

## Architecture

### Clean Modular Design
```
┌─────────────┐     ┌──────────┐     ┌─────────┐
│   Parser    │────▶│  Tokens  │────▶│ Executor│
│  (memchr)   │     │          │     │         │
└─────────────┘     └──────────┘     └────┬────┘
                                          │
                    ┌─────────────────────▼──────────────┐
                    │          Module Registry           │
                    │  ┌──────┐ ┌──────┐ ┌──────┐        │
                    │  │ Path │ │ Git  │ │ Rust │ ...    │
                    │  └──────┘ └──────┘ └──────┘        │
                    └────────────────────────────────────┘
```

### Execution Pipeline
1. **Parse** - Single-pass, zero-copy template parsing (~1µs)
2. **Registry Lookup** - O(1) module resolution via HashMap
3. **Render** - Modules validate and render in parallel
4. **Style** - ANSI styling applied as final step

### Module System
All modules implement a simple trait:
```rust
pub trait Module: Send + Sync {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String>;
}
```
- Return `Some(text)` to display
- Return `None` to hide (inactive/error)
- Validation happens during render (lazy)

### Parser Implementation
- **memchr3** for SIMD scanning of `{`, `}`, `\`
- **Zero-copy** text slices for literal sections
- **Lazy unescaping** with `Cow<str>` for efficiency
- **Single allocation** for final output string

## Building from Source

```bash
# Requirements: Rust 2024 edition
git clone https://github.com/yourusername/prmt.git
cd prmt
cargo build --release

# Run tests
cargo test

# Benchmark
./target/release/prmt --bench '{path} {rust} {git}'
```

## License

MIT
