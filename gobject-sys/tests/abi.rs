// This file was generated by gir (https://github.com/gtk-rs/gir @ 6855214)
// from gir-files (https://github.com/gtk-rs/gir-files @ cf55bdc)
// DO NOT EDIT

extern crate gobject_sys;
extern crate shell_words;
extern crate tempdir;
use std::env;
use std::error::Error;
use std::path::Path;
use std::mem::{align_of, size_of};
use std::process::Command;
use std::str;
use gobject_sys::*;

static PACKAGES: &[&str] = &["gobject-2.0"];

#[derive(Clone, Debug)]
struct Compiler {
    pub args: Vec<String>,
}

impl Compiler {
    pub fn new() -> Result<Compiler, Box<Error>> {
        let mut args = get_var("CC", "cc")?;
        args.push("-Wno-deprecated-declarations".to_owned());
        // For %z support in printf when using MinGW.
        args.push("-D__USE_MINGW_ANSI_STDIO".to_owned());
        args.extend(get_var("CFLAGS", "")?);
        args.extend(get_var("CPPFLAGS", "")?);
        args.extend(pkg_config_cflags(PACKAGES)?);
        Ok(Compiler { args })
    }

    pub fn define<'a, V: Into<Option<&'a str>>>(&mut self, var: &str, val: V) {
        let arg = match val.into() {
            None => format!("-D{}", var), 
            Some(val) => format!("-D{}={}", var, val),
        };
        self.args.push(arg);
    }

    pub fn compile(&self, src: &Path, out: &Path) -> Result<(), Box<Error>> {
        let mut cmd = self.to_command();
        cmd.arg(src);
        cmd.arg("-o");
        cmd.arg(out);
        let status = cmd.spawn()?.wait()?;
        if !status.success() {
            return Err(format!("compilation command {:?} failed, {}",
                               &cmd, status).into());
        }
        Ok(())
    }

    fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.args[0]);
        cmd.args(&self.args[1..]);
        cmd
    }
}

fn get_var(name: &str, default: &str) -> Result<Vec<String>, Box<Error>> {
    match env::var(name) {
        Ok(value) => Ok(shell_words::split(&value)?),
        Err(env::VarError::NotPresent) => Ok(shell_words::split(default)?),
        Err(err) => Err(format!("{} {}", name, err).into()),
    }
}

fn pkg_config_cflags(packages: &[&str]) -> Result<Vec<String>, Box<Error>> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }
    let mut cmd = Command::new("pkg-config");
    cmd.arg("--cflags");
    cmd.args(packages);
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(format!("command {:?} returned {}", 
                           &cmd, out.status).into());
    }
    let stdout = str::from_utf8(&out.stdout)?;
    Ok(shell_words::split(stdout.trim())?)
}


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Layout {
    size: usize,
    alignment: usize,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct Results {
    /// Number of successfully completed tests.
    passed: usize,
    /// Total number of failed tests (including those that failed to compile).
    failed: usize,
    /// Number of tests that failed to compile.
    failed_to_compile: usize,
}

impl Results {
    fn record_passed(&mut self) {
        self.passed += 1;
    }
    fn record_failed(&mut self) {
        self.failed += 1;
    }
    fn record_failed_to_compile(&mut self) {
        self.failed += 1;
        self.failed_to_compile += 1;
    }
    fn summary(&self) -> String {
        format!(
            "{} passed; {} failed (compilation errors: {})",
            self.passed,
            self.failed,
            self.failed_to_compile)
    }
    fn expect_total_success(&self) {
        if self.failed == 0 {
            println!("OK: {}", self.summary());
        } else {
            panic!("FAILED: {}", self.summary());
        };
    }
}

#[test]
fn cross_validate_constants_with_c() {
    let tmpdir = tempdir::TempDir::new("abi").expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!("1",
               get_c_value(tmpdir.path(), &cc, "1").expect("C constant"),
               "failed to obtain correct constant value for 1");

    let mut results : Results = Default::default();
    for (i, &(name, rust_value)) in RUST_CONSTANTS.iter().enumerate() {
        match get_c_value(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            },
            Ok(ref c_value) => {
                if rust_value == c_value {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!("Constant value mismatch for {}\nRust: {:?}\nC:    {:?}",
                              name, rust_value, c_value);
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("constants ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

#[test]
fn cross_validate_layout_with_c() {
    let tmpdir = tempdir::TempDir::new("abi").expect("temporary directory");
    let cc = Compiler::new().expect("configured compiler");

    assert_eq!(Layout {size: 1, alignment: 1},
               get_c_layout(tmpdir.path(), &cc, "char").expect("C layout"),
               "failed to obtain correct layout for char type");

    let mut results : Results = Default::default();
    for (i, &(name, rust_layout)) in RUST_LAYOUTS.iter().enumerate() {
        match get_c_layout(tmpdir.path(), &cc, name) {
            Err(e) => {
                results.record_failed_to_compile();
                eprintln!("{}", e);
            },
            Ok(c_layout) => {
                if rust_layout == c_layout {
                    results.record_passed();
                } else {
                    results.record_failed();
                    eprintln!("Layout mismatch for {}\nRust: {:?}\nC:    {:?}",
                              name, rust_layout, &c_layout);
                }
            }
        };
        if (i + 1) % 25 == 0 {
            println!("layout    ... {}", results.summary());
        }
    }
    results.expect_total_success();
}

fn get_c_layout(dir: &Path, cc: &Compiler, name: &str) -> Result<Layout, Box<Error>> {
    let exe = dir.join("layout");
    let mut cc = cc.clone();
    cc.define("ABI_TYPE_NAME", name);
    cc.compile(Path::new("tests/layout.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}",
                           &abi_cmd, &output).into());
    }

    let stdout = str::from_utf8(&output.stdout)?;
    let mut words = stdout.trim().split_whitespace();
    let size = words.next().unwrap().parse().unwrap();
    let alignment = words.next().unwrap().parse().unwrap();
    Ok(Layout {size, alignment})
}

fn get_c_value(dir: &Path, cc: &Compiler, name: &str) -> Result<String, Box<Error>> {
    let exe = dir.join("constant");
    let mut cc = cc.clone();
    cc.define("ABI_CONSTANT_NAME", name);
    cc.compile(Path::new("tests/constant.c"), &exe)?;

    let mut abi_cmd = Command::new(exe);
    let output = abi_cmd.output()?;
    if !output.status.success() {
        return Err(format!("command {:?} failed, {:?}",
                           &abi_cmd, &output).into());
    }

    Ok(str::from_utf8(&output.stdout)?.trim().to_owned())
}

const RUST_LAYOUTS: &[(&str, Layout)] = &[
    ("GBindingFlags", Layout {size: size_of::<GBindingFlags>(), alignment: align_of::<GBindingFlags>()}),
    ("GClosureNotifyData", Layout {size: size_of::<GClosureNotifyData>(), alignment: align_of::<GClosureNotifyData>()}),
    ("GConnectFlags", Layout {size: size_of::<GConnectFlags>(), alignment: align_of::<GConnectFlags>()}),
    ("GEnumClass", Layout {size: size_of::<GEnumClass>(), alignment: align_of::<GEnumClass>()}),
    ("GEnumValue", Layout {size: size_of::<GEnumValue>(), alignment: align_of::<GEnumValue>()}),
    ("GFlagsClass", Layout {size: size_of::<GFlagsClass>(), alignment: align_of::<GFlagsClass>()}),
    ("GFlagsValue", Layout {size: size_of::<GFlagsValue>(), alignment: align_of::<GFlagsValue>()}),
    ("GInitiallyUnowned", Layout {size: size_of::<GInitiallyUnowned>(), alignment: align_of::<GInitiallyUnowned>()}),
    ("GInitiallyUnownedClass", Layout {size: size_of::<GInitiallyUnownedClass>(), alignment: align_of::<GInitiallyUnownedClass>()}),
    ("GInterfaceInfo", Layout {size: size_of::<GInterfaceInfo>(), alignment: align_of::<GInterfaceInfo>()}),
    ("GObject", Layout {size: size_of::<GObject>(), alignment: align_of::<GObject>()}),
    ("GObjectClass", Layout {size: size_of::<GObjectClass>(), alignment: align_of::<GObjectClass>()}),
    ("GObjectConstructParam", Layout {size: size_of::<GObjectConstructParam>(), alignment: align_of::<GObjectConstructParam>()}),
    ("GParamFlags", Layout {size: size_of::<GParamFlags>(), alignment: align_of::<GParamFlags>()}),
    ("GParamSpec", Layout {size: size_of::<GParamSpec>(), alignment: align_of::<GParamSpec>()}),
    ("GParamSpecBoolean", Layout {size: size_of::<GParamSpecBoolean>(), alignment: align_of::<GParamSpecBoolean>()}),
    ("GParamSpecBoxed", Layout {size: size_of::<GParamSpecBoxed>(), alignment: align_of::<GParamSpecBoxed>()}),
    ("GParamSpecChar", Layout {size: size_of::<GParamSpecChar>(), alignment: align_of::<GParamSpecChar>()}),
    ("GParamSpecClass", Layout {size: size_of::<GParamSpecClass>(), alignment: align_of::<GParamSpecClass>()}),
    ("GParamSpecDouble", Layout {size: size_of::<GParamSpecDouble>(), alignment: align_of::<GParamSpecDouble>()}),
    ("GParamSpecEnum", Layout {size: size_of::<GParamSpecEnum>(), alignment: align_of::<GParamSpecEnum>()}),
    ("GParamSpecFlags", Layout {size: size_of::<GParamSpecFlags>(), alignment: align_of::<GParamSpecFlags>()}),
    ("GParamSpecFloat", Layout {size: size_of::<GParamSpecFloat>(), alignment: align_of::<GParamSpecFloat>()}),
    ("GParamSpecGType", Layout {size: size_of::<GParamSpecGType>(), alignment: align_of::<GParamSpecGType>()}),
    ("GParamSpecInt", Layout {size: size_of::<GParamSpecInt>(), alignment: align_of::<GParamSpecInt>()}),
    ("GParamSpecInt64", Layout {size: size_of::<GParamSpecInt64>(), alignment: align_of::<GParamSpecInt64>()}),
    ("GParamSpecLong", Layout {size: size_of::<GParamSpecLong>(), alignment: align_of::<GParamSpecLong>()}),
    ("GParamSpecObject", Layout {size: size_of::<GParamSpecObject>(), alignment: align_of::<GParamSpecObject>()}),
    ("GParamSpecOverride", Layout {size: size_of::<GParamSpecOverride>(), alignment: align_of::<GParamSpecOverride>()}),
    ("GParamSpecParam", Layout {size: size_of::<GParamSpecParam>(), alignment: align_of::<GParamSpecParam>()}),
    ("GParamSpecPointer", Layout {size: size_of::<GParamSpecPointer>(), alignment: align_of::<GParamSpecPointer>()}),
    ("GParamSpecTypeInfo", Layout {size: size_of::<GParamSpecTypeInfo>(), alignment: align_of::<GParamSpecTypeInfo>()}),
    ("GParamSpecUChar", Layout {size: size_of::<GParamSpecUChar>(), alignment: align_of::<GParamSpecUChar>()}),
    ("GParamSpecUInt", Layout {size: size_of::<GParamSpecUInt>(), alignment: align_of::<GParamSpecUInt>()}),
    ("GParamSpecUInt64", Layout {size: size_of::<GParamSpecUInt64>(), alignment: align_of::<GParamSpecUInt64>()}),
    ("GParamSpecULong", Layout {size: size_of::<GParamSpecULong>(), alignment: align_of::<GParamSpecULong>()}),
    ("GParamSpecUnichar", Layout {size: size_of::<GParamSpecUnichar>(), alignment: align_of::<GParamSpecUnichar>()}),
    ("GParamSpecValueArray", Layout {size: size_of::<GParamSpecValueArray>(), alignment: align_of::<GParamSpecValueArray>()}),
    ("GParamSpecVariant", Layout {size: size_of::<GParamSpecVariant>(), alignment: align_of::<GParamSpecVariant>()}),
    ("GParameter", Layout {size: size_of::<GParameter>(), alignment: align_of::<GParameter>()}),
    ("GSignalCMarshaller", Layout {size: size_of::<GSignalCMarshaller>(), alignment: align_of::<GSignalCMarshaller>()}),
    ("GSignalFlags", Layout {size: size_of::<GSignalFlags>(), alignment: align_of::<GSignalFlags>()}),
    ("GSignalInvocationHint", Layout {size: size_of::<GSignalInvocationHint>(), alignment: align_of::<GSignalInvocationHint>()}),
    ("GSignalMatchType", Layout {size: size_of::<GSignalMatchType>(), alignment: align_of::<GSignalMatchType>()}),
    ("GSignalQuery", Layout {size: size_of::<GSignalQuery>(), alignment: align_of::<GSignalQuery>()}),
    ("GTypeCValue", Layout {size: size_of::<GTypeCValue>(), alignment: align_of::<GTypeCValue>()}),
    ("GTypeClass", Layout {size: size_of::<GTypeClass>(), alignment: align_of::<GTypeClass>()}),
    ("GTypeDebugFlags", Layout {size: size_of::<GTypeDebugFlags>(), alignment: align_of::<GTypeDebugFlags>()}),
    ("GTypeFlags", Layout {size: size_of::<GTypeFlags>(), alignment: align_of::<GTypeFlags>()}),
    ("GTypeFundamentalFlags", Layout {size: size_of::<GTypeFundamentalFlags>(), alignment: align_of::<GTypeFundamentalFlags>()}),
    ("GTypeFundamentalInfo", Layout {size: size_of::<GTypeFundamentalInfo>(), alignment: align_of::<GTypeFundamentalInfo>()}),
    ("GTypeInfo", Layout {size: size_of::<GTypeInfo>(), alignment: align_of::<GTypeInfo>()}),
    ("GTypeInstance", Layout {size: size_of::<GTypeInstance>(), alignment: align_of::<GTypeInstance>()}),
    ("GTypeInterface", Layout {size: size_of::<GTypeInterface>(), alignment: align_of::<GTypeInterface>()}),
    ("GTypeModule", Layout {size: size_of::<GTypeModule>(), alignment: align_of::<GTypeModule>()}),
    ("GTypeModuleClass", Layout {size: size_of::<GTypeModuleClass>(), alignment: align_of::<GTypeModuleClass>()}),
    ("GTypePluginClass", Layout {size: size_of::<GTypePluginClass>(), alignment: align_of::<GTypePluginClass>()}),
    ("GTypeQuery", Layout {size: size_of::<GTypeQuery>(), alignment: align_of::<GTypeQuery>()}),
    ("GTypeValueTable", Layout {size: size_of::<GTypeValueTable>(), alignment: align_of::<GTypeValueTable>()}),
    ("GValue", Layout {size: size_of::<GValue>(), alignment: align_of::<GValue>()}),
    ("GValueArray", Layout {size: size_of::<GValueArray>(), alignment: align_of::<GValueArray>()}),
    ("GWeakRef", Layout {size: size_of::<GWeakRef>(), alignment: align_of::<GWeakRef>()}),
];

const RUST_CONSTANTS: &[(&str, &str)] = &[
    ("G_BINDING_BIDIRECTIONAL", "1"),
    ("G_BINDING_DEFAULT", "0"),
    ("G_BINDING_INVERT_BOOLEAN", "4"),
    ("G_BINDING_SYNC_CREATE", "2"),
    ("G_CONNECT_AFTER", "1"),
    ("G_CONNECT_SWAPPED", "2"),
    ("G_PARAM_CONSTRUCT", "4"),
    ("G_PARAM_CONSTRUCT_ONLY", "8"),
    ("G_PARAM_DEPRECATED", "2147483648"),
    ("G_PARAM_EXPLICIT_NOTIFY", "1073741824"),
    ("G_PARAM_LAX_VALIDATION", "16"),
    ("G_PARAM_MASK", "255"),
    ("G_PARAM_PRIVATE", "32"),
    ("G_PARAM_READABLE", "1"),
    ("G_PARAM_READWRITE", "3"),
    ("G_PARAM_STATIC_BLURB", "128"),
    ("G_PARAM_STATIC_NAME", "32"),
    ("G_PARAM_STATIC_NICK", "64"),
    ("G_PARAM_STATIC_STRINGS", "0"),
    ("G_PARAM_USER_SHIFT", "8"),
    ("G_PARAM_WRITABLE", "2"),
    ("G_SIGNAL_ACTION", "32"),
    ("G_SIGNAL_DEPRECATED", "256"),
    ("G_SIGNAL_DETAILED", "16"),
    ("G_SIGNAL_FLAGS_MASK", "511"),
    ("G_SIGNAL_MATCH_CLOSURE", "4"),
    ("G_SIGNAL_MATCH_DATA", "16"),
    ("G_SIGNAL_MATCH_DETAIL", "2"),
    ("G_SIGNAL_MATCH_FUNC", "8"),
    ("G_SIGNAL_MATCH_ID", "1"),
    ("G_SIGNAL_MATCH_MASK", "63"),
    ("G_SIGNAL_MATCH_UNBLOCKED", "32"),
    ("G_SIGNAL_MUST_COLLECT", "128"),
    ("G_SIGNAL_NO_HOOKS", "64"),
    ("G_SIGNAL_NO_RECURSE", "8"),
    ("G_SIGNAL_RUN_CLEANUP", "4"),
    ("G_SIGNAL_RUN_FIRST", "1"),
    ("G_SIGNAL_RUN_LAST", "2"),
    ("G_TYPE_DEBUG_INSTANCE_COUNT", "4"),
    ("G_TYPE_DEBUG_MASK", "7"),
    ("G_TYPE_DEBUG_NONE", "0"),
    ("G_TYPE_DEBUG_OBJECTS", "1"),
    ("G_TYPE_DEBUG_SIGNALS", "2"),
    ("G_TYPE_FLAG_ABSTRACT", "16"),
    ("G_TYPE_FLAG_CLASSED", "1"),
    ("G_TYPE_FLAG_DEEP_DERIVABLE", "8"),
    ("G_TYPE_FLAG_DERIVABLE", "4"),
    ("G_TYPE_FLAG_INSTANTIATABLE", "2"),
    ("G_TYPE_FLAG_RESERVED_ID_BIT", "1"),
    ("G_TYPE_FLAG_VALUE_ABSTRACT", "32"),
    ("G_TYPE_FUNDAMENTAL_MAX", "255"),
    ("G_TYPE_FUNDAMENTAL_SHIFT", "2"),
    ("G_TYPE_RESERVED_BSE_FIRST", "32"),
    ("G_TYPE_RESERVED_BSE_LAST", "48"),
    ("G_TYPE_RESERVED_GLIB_FIRST", "22"),
    ("G_TYPE_RESERVED_GLIB_LAST", "31"),
    ("G_TYPE_RESERVED_USER_FIRST", "49"),
    ("G_VALUE_COLLECT_FORMAT_MAX_LENGTH", "8"),
    ("G_VALUE_NOCOPY_CONTENTS", "134217728"),
];


