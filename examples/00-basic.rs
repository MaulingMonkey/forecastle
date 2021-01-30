use forecastle::d3d9;
use winapi::shared::d3d9types::D3DCLEAR_TARGET;
use std::ptr::*;



fn main() {
    d3d9::create_window("00-basic", |_| vec![
        Clear(0xFF332211),
        Clear(0xFF112233),
    ]);
    forecastle::run();
}

struct Clear(u32);

#[cfg(windows)] impl d3d9::Layer for Clear {
    fn render(&self, ctx: &mut d3d9::RenderContext) {
        unsafe { ctx.device.Clear(0, null_mut(), D3DCLEAR_TARGET, self.0, 0.0, 0) };
    }
}
