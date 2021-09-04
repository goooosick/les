fn main() {
    let data = std::fs::read("nestest.nes").expect("read rom failed");

    let mut bus = les::Bus::new();
    {
        let data = &data[0x10..][..0x4000];
        bus.load(0x8000, data);
        bus.load(0xc000, data);
    }

    let mut cpu = les::Cpu::new();
    cpu.set_pc(0xc000);

    for _ in 0.. {
        cpu.dump(&bus);
        cpu.exec(&mut bus);
    }
}
