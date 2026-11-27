/* Six-fan Rev B. Standalone STM32G0B1CB firmware, register map from RM0444. */
#include <stdint.h>
#include <stdbool.h>

/* Freestanding GCC may lower aggregate initialization to memset. */
void *memset(void *address, int value, __SIZE_TYPE__ length) {
    unsigned char *p = address;
    while (length--) *p++ = (unsigned char)value;
    return address;
}

#define REG(base, offset) (*(volatile uint32_t *)((uintptr_t)(base) + (offset)))
#define BIT(i) (UINT32_C(1) << (i))
enum { RCC = 0x40021000, FLASH_CTL = 0x40022000,
       TIM1 = 0x40012c00, TIM3 = 0x40000400, ADC = 0x40012400,
       CAN = 0x40006400, CAN_RAM = 0x4000b400, GPIO = 0x50000000 };
enum { A, B, C, D };
enum { FANS = 6, PERIOD = 2560, REPORT_MS = 500 };
typedef struct { uint8_t port, bit; } Pin;

static const Pin enable[FANS] = {{B,15},{C,6},{C,7},{A,10},{A,15},{B,3}};
static const Pin fault[FANS] = {{C,13},{C,14},{C,15},{B,12},{B,13},{B,14}};
static const Pin pwm[FANS] = {{A,6},{A,7},{B,0},{B,1},{A,8},{A,9}};
static const Pin tach[FANS] = {{D,0},{D,1},{D,2},{D,3},{B,4},{B,5}};
static volatile uint32_t ticks, edges[FANS];
static uint8_t duty[FANS], command_sequence, report_sequence, enabled, restarting;
static uint32_t restart_at[FANS], last_command;
static bool commanded;
static unsigned gpio(Pin p) { return GPIO + 0x400u * p.port; }

static void mode(Pin p, unsigned m) {
    unsigned shift = 2u * p.bit;
    REG(gpio(p), 0) = (REG(gpio(p), 0) & ~(3u << shift)) | (m << shift);
}
static void alternate(Pin p, unsigned af) {
    unsigned offset = 0x20 + 4 * (p.bit / 8), shift = 4 * (p.bit % 8);
    REG(gpio(p), offset) = (REG(gpio(p), offset) & ~(15u << shift)) | (af << shift);
    mode(p, 2);
}
static void power(unsigned i, bool on) {
    REG(gpio(enable[i]), 0x18) = BIT(enable[i].bit + (on ? 0 : 16));
    if (on) enabled |= BIT(i);
    else enabled &= ~BIT(i);
}
static void speed(unsigned i, uint8_t percent) {
    duty[i] = percent;
    /* High gate sinks the fan's PWM wire, hence the complement. */
    REG(i < 4 ? TIM3 : TIM1, 0x34 + 4 * (i < 4 ? i : i - 4)) =
        (100u - percent) * PERIOD / 100u;
}

static void clocks(void) {
    REG(FLASH_CTL, 0) = (REG(FLASH_CTL, 0) & ~7u) | 2u;
    while ((REG(FLASH_CTL, 0) & 7u) != 2u) {}
    REG(RCC, 0) |= BIT(18); /* CMOS oscillator, HSE bypass */
    REG(RCC, 0) |= BIT(16);
    while (!(REG(RCC, 0) & BIT(17))) {}
    REG(RCC, 0x0c) = 3u | (16u << 8) | BIT(28) | (1u << 29); /* 8 MHz x 16 / 2 */
    REG(RCC, 0) |= BIT(24);
    while (!(REG(RCC, 0) & BIT(25))) {}
    REG(RCC, 8) = (REG(RCC, 8) & ~7u) | 2u;
    while ((REG(RCC, 8) & (7u << 3)) != (2u << 3)) {}
    REG(RCC, 0x34) |= 15u;
    REG(RCC, 0x3c) |= BIT(1) | BIT(12); /* TIM3, FDCAN */
    REG(RCC, 0x40) |= BIT(11) | BIT(20); /* TIM1 and ADC */
    (void)REG(RCC, 0x40);
}
static void fan_io(void) {
    for (unsigned i = 0; i < FANS; ++i) {
        power(i, false);
        mode(enable[i], 1);
        mode(fault[i], 0);
        mode(tach[i], 0);
        alternate(pwm[i], i < 4 ? 1 : 2);
        speed(i, 100);
    }
    for (unsigned j = 0; j < 2; ++j) {
        unsigned timer = j ? TIM1 : TIM3;
        REG(timer, 0x2c) = PERIOD - 1;
        REG(timer, 0x18) = 0x6868; /* PWM mode 1, preload CCR1 and CCR2 */
        REG(timer, 0x1c) = 0x6868;
        REG(timer, 0x20) = j ? 0x11 : 0x1111;
        if (j) REG(timer, 0x44) = BIT(15); /* advanced timer main output */
        REG(timer, 0x14) = 1;
        REG(timer, 0) = BIT(7) | 1;
    }
}
static void tick(void) {
    static uint8_t previous;
    unsigned now = (REG(GPIO + D*0x400, 0x10) & 15u) | (REG(GPIO + B*0x400, 0x10) & 48u);
    unsigned falling = previous & ~now;
    previous = now;
    for (unsigned i = 0; i < FANS; ++i)
        if (falling & BIT(i)) ++edges[i];
    ++ticks; /* 100 us. Unsigned differences also work across rollover. */
}
static void adc_init(void) {
    REG(ADC, 0x10) = 2u << 30; /* PCLK/4 */
    REG(ADC, 8) |= BIT(28);
    for (volatile unsigned i = 0; i < 1600; ++i) {}
    REG(ADC, 8) |= BIT(31);
    while (REG(ADC, 8) & BIT(31)) {}
    for (volatile unsigned i = 0; i < 32; ++i) {}
    REG(ADC, 0x14) = 7; /* long sample time for IMON and thermistor dividers */
    REG(ADC, 8) |= 1;
    while (!(REG(ADC, 0) & 1)) {}
}
static unsigned sample(unsigned channel) {
    REG(ADC, 0) = BIT(13);
    REG(ADC, 0x28) = BIT(channel);
    while (!(REG(ADC, 0) & BIT(13))) {}
    REG(ADC, 8) |= BIT(2);
    while (!(REG(ADC, 0) & BIT(2))) {}
    return REG(ADC, 0x40);
}
static int temperature(unsigned code) {
    /* ADC codes at -20, -10, ..., 110 C, 10k pull-up and 10k NTC, B=3435 K. */
    static const uint16_t lut[] = {3627, 3368, 3037, 2654, 2248, 1854, 1496, 1191, 940, 740, 584, 462, 368, 295};
    if (code > lut[0] || code < lut[13]) return -32768;
    for (int j = 1; j <= 13; ++j) if (code >= lut[j])
        return -200 + (j-1)*100 + 100*(int)(lut[j-1]-code)/(lut[j-1]-lut[j]);
    return -32768;
}
static void can_init(void) {
    alternate((Pin){A,11}, 3);
    alternate((Pin){A,12}, 3);
    REG(CAN, 0x18) |= 1;
    while (!(REG(CAN, 0x18) & 1)) {}
    REG(CAN, 0x18) |= BIT(1);
    REG(CAN, 0x18) |= BIT(8) | BIT(9); /* FD and bit-rate switching */
    REG(CAN, 0x1c) = (2u<<25) | (7u<<16) | (11u<<8) | 2u;
    REG(CAN, 0x0c) = (1u<<16) | (11u<<8) | (2u<<4) | 2u;
    /* Dual-ID standard filter into FIFO0, reject other and remote frames. */
    REG(CAN_RAM, 0) = (1u<<30) | (1u<<27) | (0x510u<<16) | 0x512u;
    REG(CAN, 0x80) = (1u<<16) | (2u<<4) | (2u<<2) | 3u;
    REG(CAN, 0x18) &= ~1u;
    while (REG(CAN, 0x18) & 1) {}
}
static void receive(uint32_t now) {
    unsigned status = REG(CAN, 0x90);
    if (!(status & 15u)) return;
    unsigned idx = (status >> 8) & 3u;
    volatile uint32_t *msg = (volatile uint32_t *)CAN_RAM + 44 + idx*18;
    uint32_t header = msg[0], format = msg[1], lo = msg[2], hi = msg[3];
    REG(CAN, 0x94) = idx;
    if ((header & (BIT(30)|BIT(29))) || !(format & BIT(21))) return;
    unsigned id = (header >> 18) & 0x7ff, dlc = (format >> 16) & 15;
    if (id == 0x510 && dlc == 8 && (uint8_t)lo == 1) {
        uint8_t request[FANS] = {lo>>16, lo>>24, hi, hi>>8, hi>>16, hi>>24};
        for (unsigned i = 0; i < FANS; ++i) if (request[i] > 100) return;
        for (unsigned i = 0; i < FANS; ++i) speed(i, request[i]);
        command_sequence = lo >> 8;
        last_command = now;
        commanded = true;
    } else if (id == 0x512 && dlc == 3 && (lo & 0xffffu) == 0xa501) {
        unsigned mask = (lo >> 16) & 63u;
        for (unsigned i = 0; i < FANS; ++i) if (mask & enabled & BIT(i)) {
            power(i, false);
            restarting |= BIT(i);
            restart_at[i] = now;
        }
    }
}
static void put16(uint8_t *dst, int v) {
    uint16_t u = (uint16_t)v;
    dst[0] = u;
    dst[1] = u >> 8;
}
static void report(uint32_t elapsed) {
    static uint32_t previous[FANS];
    static uint8_t missed[FANS];
    uint8_t data[48] = {2, report_sequence++, !commanded, 0};
    for (unsigned i = 0; i < FANS; ++i) {
        uint32_t count = edges[i], delta = count - previous[i];
        previous[i] = count;
        if (delta || duty[i]<20 || !(enabled & BIT(i))) missed[i]=0;
        else if (missed[i]<2) ++missed[i];
        if (missed[i]==2) data[3] |= BIT(i);
        data[4+i] = duty[i];
        /* 10 kHz tick, two pulses/rev. Bound numerator for disconnected/noisy inputs. */
        uint32_t rpm = delta > 14000 ? 65535 : delta*300000u/elapsed;
        put16(data+10+2*i, rpm > 65535 ? 65535 : (int)rpm);
        put16(data+24+2*i, (sample(i)*5914u + 2047u)/4095u);
        if (!(REG(gpio(fault[i]), 0x10) & BIT(fault[i].bit))) data[23] |= BIT(i);
    }
    static const unsigned adc_ntc[] = {10,11,15};
    for (unsigned i=0;i<3;++i) put16(data+36+2*i, temperature(sample(adc_ntc[i])));
    data[22] = command_sequence;
    data[42] = enabled;
    data[43] = restarting;
    unsigned status = REG(CAN, 0xc4);
    if (status & BIT(21)) return;
    unsigned idx = (status >> 16) & 3u;
    volatile uint32_t *msg = (volatile uint32_t *)CAN_RAM + 158 + idx*18;
    msg[0] = 0x511u<<18;
    msg[1] = BIT(21) | BIT(20) | (14u<<16);
    for (unsigned i=0;i<48;i+=4)
        msg[2+i/4] = data[i] | (uint32_t)data[i+1]<<8 | (uint32_t)data[i+2]<<16 | (uint32_t)data[i+3]<<24;
    REG(CAN, 0xcc) = BIT(idx);
}
static void run(void) {
    clocks();
    fan_io();
    adc_init();
    can_init();
    REG(0xe000e010u, 4) = 6399;
    REG(0xe000e010u, 8) = 0;
    REG(0xe000e010u, 0) = 7;
    unsigned next = 0;
    uint32_t last_report = 0, last_start = 0;
    for (;;) {
        uint32_t now = ticks;
        if (next<FANS && now-last_start >= (next ? 2000u : 5000u)) {
            power(next++, true);
            last_start = now;
        }
        for (unsigned i=0;i<FANS;++i) if ((restarting & BIT(i)) && now-restart_at[i]>=1000u) {
            power(i,true);
            restarting &= ~BIT(i);
        }
        if (commanded && now-last_command >= 10000u) {
            for (unsigned i=0;i<FANS;++i) speed(i,100);
            commanded=false;
        }
        if (REG(CAN, 0x44) & BIT(7)) REG(CAN, 0x18) &= ~1u; /* bus-off recovery */
        receive(now);
        if (now-last_report >= REPORT_MS*10) {
            report(now-last_report);
            last_report = now;
        }
    }
}
extern uint32_t _stack, _idata, _data, _edata, _bss, _ebss;
void reset(void) {
    uint32_t *src = &_idata;
    for (uint32_t *p=&_data;p<&_edata;) *p++=*src++;
    for (uint32_t *p=&_bss;p<&_ebss;) *p++=0;
    run();
}
static void halt(void) { for (;;) {} }
__attribute__((section(".vectors"),used)) void (*const vectors[16])(void) = {
    (void(*)(void))&_stack,reset,halt,halt,0,0,0,0,0,0,0,halt,0,0,halt,tick
};
