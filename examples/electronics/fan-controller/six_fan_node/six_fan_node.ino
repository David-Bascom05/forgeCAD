// Teensy 4.1 carrier. Hardware pin numbers match U3 on controller.fg.
#include <Arduino.h>
#include <FlexCAN_T4.h>

constexpr uint8_t fanCount = 6;
constexpr uint8_t pwmPin[fanCount] = {2, 3, 4, 5, 6, 9};
constexpr uint8_t tachPin[fanCount] = {14, 15, 16, 17, 18, 19};
constexpr uint8_t pulsesPerRevolution[fanCount] = {2, 2, 2, 2, 2, 2};
constexpr uint32_t commandId = 0x510;
constexpr uint32_t telemetryId = 0x511;
constexpr uint32_t commandTimeoutMs = 1000;
constexpr uint32_t reportPeriodMs = 500;

FlexCAN_T4FD<CAN3, RX_SIZE_64, TX_SIZE_16> bus;
volatile uint32_t pulseCount[fanCount] = {};
uint8_t appliedDuty[fanCount] = {};
uint8_t emptyWindows[fanCount] = {};
uint8_t commandSequence = 0;
uint8_t reportSequence = 0;
uint32_t commandTime = 0;
uint32_t windowStart = 0;
bool failsafe = true;
bool busReady = false;

template <unsigned Channel> void countPulse() {
    ++pulseCount[Channel];
}
using Interrupt = void (*)();
constexpr Interrupt tachInterrupt[fanCount] = {
    countPulse<0>, countPulse<1>, countPulse<2>, countPulse<3>, countPulse<4>, countPulse<5>
};

void setDuty(unsigned channel, uint8_t percent) {
    // The low-side FET inverts the drive. A low gate releases the fan input.
    if (percent < 20 || appliedDuty[channel] < 20) emptyWindows[channel] = 0;
    appliedDuty[channel] = percent;
    analogWrite(pwmPin[channel], ((100 - percent) * 255 + 50) / 100);
}

void fullCooling() {
    for (unsigned i = 0; i < fanCount; ++i) setDuty(i, 100);
    failsafe = true;
}

void setup() {
    analogWriteResolution(8);
    for (unsigned i = 0; i < fanCount; ++i) {
        digitalWrite(pwmPin[i], LOW);
        pinMode(pwmPin[i], OUTPUT);
        analogWriteFrequency(pwmPin[i], 25000);
        pinMode(tachPin[i], INPUT); // External 4.7 k pull-up to 3.3 V.
        attachInterrupt(digitalPinToInterrupt(tachPin[i]), tachInterrupt[i], FALLING);
    }
    fullCooling();
    bus.begin();
    CANFD_timings_t timing;
    timing.clock = CLK_24MHz;
    timing.baudrate = 500000;
    timing.baudrateFD = 2000000;
    timing.propdelay = 190;
    timing.bus_length = 1;
    timing.sample = 75;
    busReady = bus.setBaudRate(timing);
    bus.setRegions(32);
    windowStart = millis();
}

void acceptCommand(const CANFD_message_t &frame, uint32_t now) {
    if (frame.id != commandId || frame.flags.extended || !frame.edl ||
        frame.len != 8 || frame.buf[0] != 1) return;
    for (unsigned i = 0; i < fanCount; ++i) {
        if (frame.buf[i + 2] > 100) return;
    }
    for (unsigned i = 0; i < fanCount; ++i) setDuty(i, frame.buf[i + 2]);
    commandSequence = frame.buf[1];
    commandTime = now;
    failsafe = false;
}

void report(uint32_t now) {
    uint32_t counts[fanCount];
    noInterrupts();
    for (unsigned i = 0; i < fanCount; ++i) {
        counts[i] = pulseCount[i];
        pulseCount[i] = 0;
    }
    interrupts();
    const uint32_t elapsed = now - windowStart;
    windowStart = now;
    CANFD_message_t frame;
    frame.id = telemetryId;
    frame.len = 24;
    frame.edl = true;
    frame.brs = true;
    frame.buf[0] = 1;
    frame.buf[1] = reportSequence++;
    frame.buf[2] = failsafe ? 1 : 0;
    for (unsigned i = 0; i < fanCount; ++i) {
        if (counts[i] || appliedDuty[i] < 20) emptyWindows[i] = 0;
        else if (emptyWindows[i] < 2) ++emptyWindows[i];
        if (emptyWindows[i] == 2) frame.buf[3] |= 1u << i;
        frame.buf[4 + i] = appliedDuty[i];
        const uint64_t rpm = uint64_t(counts[i]) * 60000 /
                             (uint64_t(elapsed) * pulsesPerRevolution[i]);
        const uint16_t value = rpm > 65535 ? 65535 : uint16_t(rpm);
        frame.buf[10 + 2 * i] = uint8_t(value);
        frame.buf[11 + 2 * i] = uint8_t(value >> 8);
    }
    frame.buf[22] = commandSequence;
    if (busReady) bus.write(frame);
}

void loop() {
    const uint32_t now = millis();
    CANFD_message_t frame;
    // Bound polling even if other nodes saturate the bus.
    for (unsigned i = 0; busReady && i < 16 && bus.read(frame); ++i) acceptCommand(frame, now);
    if (!failsafe && now - commandTime >= commandTimeoutMs) fullCooling();
    if (now - windowStart >= reportPeriodMs) report(now);
}
