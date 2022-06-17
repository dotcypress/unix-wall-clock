#![no_std]
#![no_main]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

mod display;
mod remote;
mod watch;

use defmt_rtt as _;

use hal::gpio::*;
use hal::prelude::*;
use hal::spi::*;
use hal::stm32;
use hal::timer::*;
use infrared::protocols::Nec;
use infrared::PeriodicReceiver;
use watch::*;

pub type Ir = PeriodicReceiver<Nec, gpioc::PC15<Input<Floating>>>;
pub type Display = display::DisplayController<
    Spi<stm32::SPI2, (gpiob::PB8<Analog>, NoMiso, gpioa::PA4<Analog>)>,
    gpioa::PA3<Output<PushPull>>,
>;

#[rtic::app(device = stm32, peripherals = true)]
mod app {
    use super::*;

    #[local]
    struct Local {
        ir: Ir,
        animation_timer: Timer<stm32::TIM14>,
        clock_timer: Timer<stm32::TIM17>,
        ir_timer: Timer<stm32::TIM16>,
    }

    #[shared]
    struct Shared {
        watch: Watch,
        display: Display,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(hal::rcc::Config::pll());

        let mut ir_timer = ctx.device.TIM16.timer(&mut rcc);
        ir_timer.start(20.khz());
        ir_timer.listen();

        let mut clock_timer = ctx.device.TIM17.timer(&mut rcc);
        clock_timer.start(1.hz());
        clock_timer.listen();

        let mut animation_timer = ctx.device.TIM14.timer(&mut rcc);
        animation_timer.start(8.hz());
        animation_timer.listen();

        let port_a = ctx.device.GPIOA.split(&mut rcc);
        let port_b = ctx.device.GPIOB.split(&mut rcc);
        let port_c = ctx.device.GPIOC.split(&mut rcc);

        let spi = ctx.device.SPI2.spi(
            (port_b.pb8, NoMiso, port_a.pa4),
            hal::spi::MODE_0,
            10.mhz(),
            &mut rcc,
        );
        port_a.pa5.into_push_pull_output().set_low().unwrap();
        let mut display = Display::new(spi, port_a.pa3.into());

        let watch = Watch::new();
        watch.animate(&mut display);
        display.render();

        let ir = PeriodicReceiver::new(port_c.pc15.into(), 20_000);

        (
            Shared { display, watch },
            Local {
                ir,
                animation_timer,
                ir_timer,
                clock_timer,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM16, local = [ir, ir_timer], shared = [watch])]
    fn ir_poll(mut ctx: ir_poll::Context) {
        if let Ok(Some(cmd)) = ctx.local.ir.poll() {
            ctx.shared.watch.lock(|watch| watch.ir_command(cmd));
        }
        ctx.local.ir_timer.clear_irq();
    }

    #[task(binds = TIM17, local = [clock_timer], shared = [watch])]
    fn tick(mut ctx: tick::Context) {
        ctx.shared.watch.lock(|watch| watch.tick());
        ctx.local.clock_timer.clear_irq();
    }

    #[task(binds = TIM14, priority = 2, local = [animation_timer], shared = [display, watch])]
    fn animate(ctx: animate::Context) {
        (ctx.shared.display, ctx.shared.watch).lock(|display, watch| watch.animate(display));
        ctx.local.animation_timer.clear_irq();
    }

    #[idle(shared = [display])]
    fn idle(mut ctx: idle::Context) -> ! {
        loop {
            ctx.shared.display.lock(|display| display.render());
        }
    }
}
