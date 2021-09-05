fn main() {
    let cart = les::Cartridge::load("nestest.nes").expect("load rom failed");
    let mut bus = les::Bus::new(cart);

    let mut cpu = les::Cpu::default();
    cpu.reset(&mut bus);

    for _ in 0.. {
        cpu.dump(&bus);
        cpu.exec(&mut bus);
    }
}
