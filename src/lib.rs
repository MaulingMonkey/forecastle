#![allow(unused_unsafe)]
#![deny(unreachable_patterns)]



#[path ="d3d9/_d3d9.rs"]        pub mod d3d9;
#[path ="windows/_windows.rs"]  pub mod windows;
pub mod update;



pub fn run() {
    #[cfg(windows)] crate::windows::run();
}
