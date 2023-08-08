#![no_std]
#![no_main]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

mod display;
mod watch;

use defmt_rtt as _;

use hal::gpio::*;
use hal::prelude::*;
use hal::serial::*;
use hal::spi::*;
use hal::stm32;
use hal::timer::*;
use infrared::protocols::Nec;
use infrared::PeriodicReceiver;
use watch::*;

pub type Ir = PeriodicReceiver<Nec, gpioc::PC15<Input<Floating>>>;
pub type Display = display::DisplayController<
    Spi<stm32::SPI2, (gpiob::PB8<Analog>, NoMiso, gpioa::PA4<Analog>)>,
    gpioa::PA6<Output<PushPull>>,
>;

#[rtic::app(device = stm32, peripherals = true)]
mod app {
    use super::*;

    #[local]
    struct Local {
        ir: Ir,
        animation_timer: Timer<stm32::TIM14>,
        ir_timer: Timer<stm32::TIM16>,
        uart: Serial<stm32::USART2, BasicConfig>,
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

        let mut animation_timer = ctx.device.TIM14.timer(&mut rcc);
        animation_timer.start(20.hz());
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
        let mut display = Display::new(spi, port_a.pa6.into());

        let mut watch = Watch::new();
        watch.animate(&mut display);
        display.render();

        let ir = PeriodicReceiver::new(port_c.pc15.into(), 20_000);

        let uart_cfg = BasicConfig::default().baudrate(9600.bps());
        let mut uart = ctx
            .device
            .USART2
            .usart(port_a.pa2, port_a.pa3, uart_cfg, &mut rcc)
            .expect("Failed to init serial port");
        uart.listen(Event::Rxne);

        (
            Shared { display, watch },
            Local {
                ir,
                animation_timer,
                ir_timer,
                uart,
            },
            init::Monotonics(),
        )
    }

    #[task(
        binds = USART2,
        shared = [watch],
        local = [
            uart,
            needle: usize = 0,
            scratch: [u8; 255] = [0; 255],
        ]
    )]
    fn uart_rx(mut ctx: uart_rx::Context) {
        loop {
            match ctx.local.uart.read() {
                Err(hal::nb::Error::WouldBlock) => return,
                Err(hal::nb::Error::Other(_)) => {
                    defmt::error!("UART RX ERROR");
                }
                Ok(b'\n') => {
                    if let Ok(line) = core::str::from_utf8(&ctx.local.scratch[..*ctx.local.needle])
                    {
                        if line.contains("ANTENNA OPEN") {
                            defmt::info!("ANTENNA OPEN");
                        } else if line.contains("ANTENNA OK") {
                            defmt::info!("ANTENNA OK");
                        } else if line.starts_with("$GNZDA") {
                            fn parse(s: Option<&str>) -> u64 {
                                s.unwrap_or("").parse::<u64>().unwrap_or(0)
                            }
                            // $GNZDA,004101.000,08,08,2023,00,00*4F
                            let mut chunks = line.split(',').skip(1);
                            let time = parse(chunks.next().unwrap_or("").split('.').next());
                            let seconds = time % 100;
                            let minutes = (time / 100) % 100;
                            let hours = (time / 10_000) % 100;
                            let day = parse(chunks.next());
                            let month = parse(chunks.next());
                            let year = parse(chunks.next());
                            ctx.shared.watch.lock(|watch| {
                                watch.set_utc_time(year, month, day, hours, minutes, seconds)
                            });
                        }
                    }
                    *ctx.local.needle = 0;
                }
                Ok(b) => {
                    ctx.local.scratch[*ctx.local.needle] = b;
                    *ctx.local.needle += 1;
                }
            }
        }
    }

    #[task(binds = TIM16, local = [ir, ir_timer], shared = [watch])]
    fn ir_poll(mut ctx: ir_poll::Context) {
        if let Ok(Some(cmd)) = ctx.local.ir.poll() {
            ctx.shared.watch.lock(|watch| watch.ir_command(cmd));
        }
        ctx.local.ir_timer.clear_irq();
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
