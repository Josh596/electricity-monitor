/*********
  Rui Santos & Sara Santos - Random Nerd Tutorials
  Complete project details at https://RandomNerdTutorials.com/getting-started-esp32-c3-super-mini/
  Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files.
  The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
*********/

#include <WiFi.h>
#include <inttypes.h>
#include <esp_http_client.h>
#include <esp_crt_bundle.h>

// SSID, WIFI_PASSWORD, API_KEY and SERVER_ENDPOINT
#include "secrets.h"

const char* DEVICE_ID = "homev1";
const int REQUEST_INTERVAL_SECS = 30;

uint32_t boot_id;
const char* reset_reason;
unsigned long last_request_time = 0;
bool has_sent_request = false;


void setupWifi() {
  Serial.print("Connecting to ");
  Serial.println(SSID);
  WiFi.begin(SSID, WIFI_PASSWORD);
  WiFi.setTxPower(WIFI_POWER_8_5dBm);
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }
  Serial.println("");
  Serial.println("WiFi connected.");
  Serial.println("IP address: ");
  Serial.println(WiFi.localIP());
}

const char* getReasonForReset() {
  esp_reset_reason_t reason = esp_reset_reason();

  switch (reason) {
    case esp_reset_reason_t::ESP_RST_BROWNOUT:
      return "brownout";
    case esp_reset_reason_t::ESP_RST_POWERON:
      return "power_on";
    case esp_reset_reason_t::ESP_RST_PANIC:
      return "panic";
    case esp_reset_reason_t::ESP_RST_INT_WDT:
    case esp_reset_reason_t::ESP_RST_TASK_WDT:
    case esp_reset_reason_t::ESP_RST_WDT:
      return "watchdog";
    case esp_reset_reason_t::ESP_RST_SW:
      return "software";
    case esp_reset_reason_t::ESP_RST_EXT:
      return "external";
    default:
      return "unknown";
  }
}

void sendRequest() {
  uint32_t uptime_ms = millis();

  // Construct the JSON payload
  char body[256];
  int len = snprintf(body, sizeof(body),
    "{\"device_id\":\"%s\",\"boot_id\":%" PRIu32 ",\"reset_reason\":\"%s\",\"uptime_ms\":%" PRIu32 "}",
    DEVICE_ID, boot_id, reset_reason, uptime_ms);

  if (len < 0 || len >= (int)sizeof(body)) {
    Serial.println("JSON body too long or formatting error");
    return;
  }

  Serial.println(body);

  if (WiFi.status() != WL_CONNECTED) {
    Serial.println("WiFi not connected, skipping beat");
    return;
  }

  // Send the HTTP POST request to the server
  esp_http_client_config_t config = {};
  config.url = SERVER_ENDPOINT;
  config.method = HTTP_METHOD_POST;
  config.timeout_ms = 5000;
  config.crt_bundle_attach = esp_crt_bundle_attach;  // verify HTTPS certificates

  esp_http_client_handle_t client = esp_http_client_init(&config);
  if (client == NULL) {
    Serial.println("Failed to create HTTP client");
    return;
  }

  esp_http_client_set_header(client, "Content-Type", "application/json");
  // Add the authentication header
  esp_http_client_set_header(client, "Authorization", "Bearer " API_KEY);
  esp_http_client_set_post_field(client, body, len);

  esp_err_t err = esp_http_client_perform(client);
  if (err == ESP_OK) {
    int status = esp_http_client_get_status_code(client);
    if (status == 204) {
      Serial.println("Beat accepted");
    } else {
      Serial.printf("Server responded with HTTP %d\n", status);
    }
  } else {
    Serial.printf("Request failed: %s\n", esp_err_to_name(err));
  }

  esp_http_client_cleanup(client);
}

void setup() {
  Serial.begin(115200);

  // Connect to Wi-Fi network
  setupWifi();

  boot_id = esp_random();
  Serial.printf("Boot ID: %" PRIu32 "\n", boot_id);

  reset_reason = getReasonForReset();
  Serial.printf("Reset reason: %s\n", reset_reason);
}



void loop() {
  // Track time since last request; the first beat goes out immediately
  bool should_send_request = !has_sent_request || millis() - last_request_time >= REQUEST_INTERVAL_SECS * 1000UL;

  if (should_send_request) {
    sendRequest();
    last_request_time = millis();
    has_sent_request = true;
  }
}
