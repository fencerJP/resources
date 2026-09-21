cargo_env = environment()

# App specific env vars
cargo_env.set('APP_ID', application_id)
cargo_env.set('DEVLOPMENT_BUILD', development_build.to_string())
cargo_env.set('VERSION', version + version_suffix)
cargo_env.set('GETTEXT_PACKAGE', gettext_package)
cargo_env.set('LOCALEDIR', localedir)
cargo_env.set('LIBEXECDIR', libexecdir)
cargo_env.set('RESOURCES_FILE', pkgdatadir / 'resources.gresource')

# Cargo env vars
cargo_env.append('CARGO_HOME', meson.project_build_root() / 'cargo')

rust_target = profile
if get_option('profile') == 'dev'
  rust_target = 'debug'
endif

cargo_options = [
  '--manifest-path', meson.project_source_root() / 'Cargo.toml',
  '--target-dir', meson.project_build_root() / 'src',
  '--profile', profile
]

test(
  'Cargo tests (main application)',
  cargo,
  args: ['test', cargo_options],
  timeout: 3600,
  env: cargo_env,
)

cargo_build = custom_target(
  'cargo-build',
  depends: resources,
  build_by_default: true,
  build_always_stale: true,
  output: rust_target,
  console: true,
  env: cargo_env,
  command: [
    cargo,
    'build',
    cargo_options,
  ],
)

copy_binary = custom_target(
  'cp-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: bindir,
  output: meson.project_name(),
  command: [
    'cp',
    'src' / rust_target / meson.project_name(),
    '@OUTPUT@',
  ],
)

copy_kill_binary = custom_target(
  'cp-kill-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  output: meson.project_name() + '-kill',
  command: [
    'cp',
    'src' / rust_target / meson.project_name() + '-kill',
    '@OUTPUT@',
  ],
)

copy_processes_binary = custom_target(
  'cp-processes-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  output: meson.project_name() + '-processes',
  command: [
    'cp',
    'src' / rust_target / meson.project_name() + '-processes',
    '@OUTPUT@',
  ],
)

copy_adjust_binary = custom_target(
  'cp-adjust-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: libexecdir,
  output: meson.project_name() + '-adjust',
  command: [
    'cp',
    'src' / rust_target / meson.project_name() + '-adjust',
    '@OUTPUT@',
  ],
)

copy_exporter_binary = custom_target(
  'cp-exporter-binary',
  depends: cargo_build,
  build_by_default: true,
  build_always_stale: true,
  install: true,
  install_dir: bindir,
  output: 'npu-data-exporter',
  command: [
    'cp',
    'src' / rust_target / 'npu-data-exporter',
    '@OUTPUT@',
  ],
)
