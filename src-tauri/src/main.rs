// Prevents console window on Windows in all build modes (both dev and release)
#![windows_subsystem = "windows"]

fn main() {
  app_lib::run();
}
