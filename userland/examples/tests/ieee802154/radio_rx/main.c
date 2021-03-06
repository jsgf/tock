#include <stdbool.h>
#include <stdio.h>

#include <led.h>
#include <radio.h>
#include <timer.h>

#define RADIO_FRAME_SIZE 129
#define BUF_SIZE 60
char packet_rx[RADIO_FRAME_SIZE];
char packet_tx[BUF_SIZE];
bool toggle = true;

static void callback(__attribute__ ((unused)) int err,
                     __attribute__ ((unused)) int payload_offset,
                     __attribute__ ((unused)) int payload_length,
                     __attribute__ ((unused)) void* ud) {
  led_toggle(0);

#define PRINT_PAYLOAD 0
#if PRINT_PAYLOAD
  printf("Received packet with payload of %d bytes from offset %d\n", payload_length, payload_offset);
  int i;
  for (i = 0; i < payload_length; i++) {
    printf("%02x%c", packet_rx[payload_offset + i],
           ((i + 1) % 16 == 0 || i + 1 == payload_length) ? '\n' : ' ');
  }
#endif

  radio_receive_callback(callback, packet_rx, RADIO_FRAME_SIZE);
}

int main(void) {
  int i;
  /* printf("Starting 802.15.4 packet reception app.\n"); */
  for (i = 0; i < RADIO_FRAME_SIZE; i++) {
    packet_rx[i] = 0;
  }
  for (i = 0; i < BUF_SIZE; i++) {
    packet_tx[i] = i;
  }
  radio_set_addr(0x802);
  radio_set_pan(0xABCD);
  radio_commit();
  radio_init();
  radio_receive_callback(callback, packet_rx, RADIO_FRAME_SIZE);
  while (1) {
    delay_ms(4000);
  }
}
