/*
 * A thin layer over rc_client for the Rust side: plain functions and flat records, so that no
 * rcheevos structure layout has to be mirrored in Rust. Every call is made from the thread that
 * owns the client.
 */

#include "rc_client.h"
#include "rc_consoles.h"
#include "rc_hash.h"

#include <stdlib.h>
#include <string.h>

/* Events of the shim, after the rc_client ones. */
#define ZAX_RA_EVENT_LOGIN 100
#define ZAX_RA_EVENT_GAME_LOADED 101

typedef struct zax_ra_event_t {
  uint32_t kind;
  int32_t result;
  uint32_t id;
  uint32_t number;
  const char* title;
  const char* description;
  const char* detail;
  const char* badge_url;
} zax_ra_event_t;

typedef struct zax_ra_entry_t {
  uint32_t bucket;
  const char* bucket_label;
  uint32_t id;
  uint32_t points;
  const char* title;
  const char* description;
  const char* progress;
  float percent;
  uint32_t unlocked;
  uint32_t state;
  uint32_t type;
  float rarity;
  const char* badge_url;
  const char* badge_locked_url;
} zax_ra_entry_t;

typedef uint32_t (*zax_ra_read_fn)(void* host, uint32_t address, uint8_t* buffer, uint32_t count);
typedef void (*zax_ra_request_fn)(void* host, void* pending, const char* url, const char* post_data,
                                  const char* content_type);
typedef void (*zax_ra_event_fn)(void* host, const zax_ra_event_t* event);
typedef void (*zax_ra_log_fn)(void* host, const char* message);
typedef void (*zax_ra_visit_fn)(void* context, const zax_ra_entry_t* entry);

typedef struct zax_ra_t {
  rc_client_t* client;
  void* host;
  zax_ra_read_fn read;
  zax_ra_request_fn request;
  zax_ra_event_fn event;
  zax_ra_log_fn log;
} zax_ra_t;

typedef struct zax_ra_pending_t {
  rc_client_server_callback_t callback;
  void* callback_data;
} zax_ra_pending_t;

static zax_ra_t* zax_ra_of(const rc_client_t* client) {
  return (zax_ra_t*)rc_client_get_userdata(client);
}

static uint32_t RC_CCONV zax_ra_read_memory(uint32_t address, uint8_t* buffer, uint32_t num_bytes,
                                            rc_client_t* client) {
  zax_ra_t* ra = zax_ra_of(client);
  if (!ra)
    return 0;
  return ra->read(ra->host, address, buffer, num_bytes);
}

static void RC_CCONV zax_ra_server_call(const rc_api_request_t* request,
                                        rc_client_server_callback_t callback, void* callback_data,
                                        rc_client_t* client) {
  zax_ra_t* ra = zax_ra_of(client);
  zax_ra_pending_t* pending = (zax_ra_pending_t*)malloc(sizeof(zax_ra_pending_t));
  if (!ra || !pending) {
    rc_api_server_response_t response;
    memset(&response, 0, sizeof(response));
    response.http_status_code = RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR;
    free(pending);
    callback(&response, callback_data);
    return;
  }
  pending->callback = callback;
  pending->callback_data = callback_data;
  ra->request(ra->host, pending, request->url, request->post_data, request->content_type);
}

static void RC_CCONV zax_ra_log_message(const char* message, const rc_client_t* client) {
  zax_ra_t* ra = zax_ra_of(client);
  if (ra)
    ra->log(ra->host, message);
}

static void zax_ra_emit(zax_ra_t* ra, zax_ra_event_t* event) {
  if (ra)
    ra->event(ra->host, event);
}

static void RC_CCONV zax_ra_event_handler(const rc_client_event_t* source, rc_client_t* client) {
  zax_ra_event_t event;
  memset(&event, 0, sizeof(event));
  event.kind = source->type;
  switch (source->type) {
    case RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED:
    case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_SHOW:
    case RC_CLIENT_EVENT_ACHIEVEMENT_CHALLENGE_INDICATOR_HIDE:
    case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_SHOW:
    case RC_CLIENT_EVENT_ACHIEVEMENT_PROGRESS_INDICATOR_UPDATE:
      if (source->achievement) {
        event.id = source->achievement->id;
        event.number = source->achievement->points;
        event.title = source->achievement->title;
        event.description = source->achievement->description;
        event.detail = source->achievement->measured_progress;
        event.badge_url = source->achievement->badge_url;
      }
      break;
    case RC_CLIENT_EVENT_LEADERBOARD_STARTED:
    case RC_CLIENT_EVENT_LEADERBOARD_FAILED:
    case RC_CLIENT_EVENT_LEADERBOARD_SUBMITTED:
      if (source->leaderboard) {
        event.id = source->leaderboard->id;
        event.title = source->leaderboard->title;
        event.description = source->leaderboard->description;
        event.detail = source->leaderboard->tracker_value;
      }
      break;
    case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_SHOW:
    case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_HIDE:
    case RC_CLIENT_EVENT_LEADERBOARD_TRACKER_UPDATE:
      if (source->leaderboard_tracker) {
        event.id = source->leaderboard_tracker->id;
        event.detail = source->leaderboard_tracker->display;
      }
      break;
    case RC_CLIENT_EVENT_LEADERBOARD_SCOREBOARD:
      if (source->leaderboard_scoreboard) {
        event.id = source->leaderboard_scoreboard->leaderboard_id;
        event.number = source->leaderboard_scoreboard->new_rank;
        event.detail = source->leaderboard_scoreboard->submitted_score;
      }
      break;
    case RC_CLIENT_EVENT_SERVER_ERROR:
      if (source->server_error) {
        event.result = source->server_error->result;
        event.id = source->server_error->related_id;
        event.title = source->server_error->api;
        event.detail = source->server_error->error_message;
      }
      break;
    case RC_CLIENT_EVENT_SUBSET_COMPLETED:
      if (source->subset) {
        event.id = source->subset->id;
        event.title = source->subset->title;
        event.badge_url = source->subset->badge_url;
      }
      break;
    default:
      break;
  }
  zax_ra_emit(zax_ra_of(client), &event);
}

static void RC_CCONV zax_ra_callback(uint32_t kind, int result, const char* error_message,
                                     rc_client_t* client) {
  zax_ra_event_t event;
  memset(&event, 0, sizeof(event));
  event.kind = kind;
  event.result = result;
  event.detail = error_message;
  zax_ra_emit(zax_ra_of(client), &event);
}

static void RC_CCONV zax_ra_login_done(int result, const char* error_message, rc_client_t* client,
                                       void* userdata) {
  (void)userdata;
  zax_ra_callback(ZAX_RA_EVENT_LOGIN, result, error_message, client);
}

static void RC_CCONV zax_ra_game_done(int result, const char* error_message, rc_client_t* client,
                                      void* userdata) {
  (void)userdata;
  zax_ra_callback(ZAX_RA_EVENT_GAME_LOADED, result, error_message, client);
}

zax_ra_t* zax_ra_create(void* host, zax_ra_read_fn read, zax_ra_request_fn request,
                        zax_ra_event_fn event, zax_ra_log_fn log, int log_level) {
  zax_ra_t* ra = (zax_ra_t*)calloc(1, sizeof(zax_ra_t));
  if (!ra)
    return NULL;
  ra->host = host;
  ra->read = read;
  ra->request = request;
  ra->event = event;
  ra->log = log;
  ra->client = rc_client_create(zax_ra_read_memory, zax_ra_server_call);
  if (!ra->client) {
    free(ra);
    return NULL;
  }
  rc_client_set_userdata(ra->client, ra);
  rc_client_set_event_handler(ra->client, zax_ra_event_handler);
  rc_client_enable_logging(ra->client, log_level, zax_ra_log_message);
  return ra;
}

void zax_ra_destroy(zax_ra_t* ra) {
  if (!ra)
    return;
  rc_client_destroy(ra->client);
  free(ra);
}

void zax_ra_complete(zax_ra_pending_t* pending, const char* body, size_t body_length, int status) {
  rc_api_server_response_t response;
  rc_client_server_callback_t callback = pending->callback;
  void* callback_data = pending->callback_data;
  free(pending);
  response.body = body;
  response.body_length = body_length;
  response.http_status_code = status;
  callback(&response, callback_data);
}

void zax_ra_discard(zax_ra_pending_t* pending) {
  free(pending);
}

size_t zax_ra_user_agent_clause(zax_ra_t* ra, char* buffer, size_t size) {
  return rc_client_get_user_agent_clause(ra->client, buffer, size);
}

void zax_ra_set_host(zax_ra_t* ra, const char* host) {
  rc_client_set_host(ra->client, host);
}

void zax_ra_set_modes(zax_ra_t* ra, int hardcore, int encore, int unofficial) {
  rc_client_set_hardcore_enabled(ra->client, hardcore);
  rc_client_set_encore_mode_enabled(ra->client, encore);
  rc_client_set_unofficial_enabled(ra->client, unofficial);
}

void zax_ra_login_password(zax_ra_t* ra, const char* username, const char* password) {
  rc_client_begin_login_with_password(ra->client, username, password, zax_ra_login_done, NULL);
}

void zax_ra_login_token(zax_ra_t* ra, const char* username, const char* token) {
  rc_client_begin_login_with_token(ra->client, username, token, zax_ra_login_done, NULL);
}

void zax_ra_logout(zax_ra_t* ra) {
  rc_client_logout(ra->client);
}

int zax_ra_user(zax_ra_t* ra, const char** display_name, const char** username, const char** token,
                uint32_t* score, uint32_t* score_softcore) {
  const rc_client_user_t* user = rc_client_get_user_info(ra->client);
  if (!user)
    return 0;
  *display_name = user->display_name;
  *username = user->username;
  *token = user->token;
  *score = user->score;
  *score_softcore = user->score_softcore;
  return 1;
}

void zax_ra_load_game(zax_ra_t* ra, const uint8_t* data, size_t size, const char* path) {
  rc_client_begin_identify_and_load_game(ra->client, RC_CONSOLE_NINTENDO, path, data, size,
                                         zax_ra_game_done, NULL);
}

/* The hash RetroAchievements knows a NES ROM by (an iNES file: its header is left out), without
 * a client: the list of the games studies them. 1 when `hash` (33 bytes) holds it. */
int zax_ra_hash_nes(const uint8_t* data, size_t size, char* hash) {
  return rc_hash_generate_from_buffer(hash, RC_CONSOLE_NINTENDO, data, size);
}

void zax_ra_unload_game(zax_ra_t* ra) {
  rc_client_unload_game(ra->client);
}

int zax_ra_load_game_state(zax_ra_t* ra) {
  return rc_client_get_load_game_state(ra->client);
}

int zax_ra_game(zax_ra_t* ra, uint32_t* id, const char** title, const char** hash,
                const char** badge_url) {
  const rc_client_game_t* game = rc_client_get_game_info(ra->client);
  if (!game)
    return 0;
  *id = game->id;
  *title = game->title;
  *hash = game->hash;
  *badge_url = game->badge_url;
  return 1;
}

void zax_ra_summary(zax_ra_t* ra, uint32_t* core, uint32_t* unofficial, uint32_t* unlocked,
                    uint32_t* unsupported, uint32_t* points_core, uint32_t* points_unlocked) {
  rc_client_user_game_summary_t summary;
  memset(&summary, 0, sizeof(summary));
  rc_client_get_user_game_summary(ra->client, &summary);
  *core = summary.num_core_achievements;
  *unofficial = summary.num_unofficial_achievements;
  *unlocked = summary.num_unlocked_achievements;
  *unsupported = summary.num_unsupported_achievements;
  *points_core = summary.points_core;
  *points_unlocked = summary.points_unlocked;
}

void zax_ra_do_frame(zax_ra_t* ra) {
  rc_client_do_frame(ra->client);
}

void zax_ra_idle(zax_ra_t* ra) {
  rc_client_idle(ra->client);
}

void zax_ra_reset(zax_ra_t* ra) {
  rc_client_reset(ra->client);
}

size_t zax_ra_progress_size(zax_ra_t* ra) {
  return rc_client_progress_size(ra->client);
}

int zax_ra_serialize_progress(zax_ra_t* ra, uint8_t* buffer, size_t size) {
  return rc_client_serialize_progress_sized(ra->client, buffer, size);
}

int zax_ra_deserialize_progress(zax_ra_t* ra, const uint8_t* data, size_t size) {
  return rc_client_deserialize_progress_sized(ra->client, data, size);
}

size_t zax_ra_rich_presence(zax_ra_t* ra, char* buffer, size_t size) {
  if (!rc_client_has_rich_presence(ra->client))
    return 0;
  return rc_client_get_rich_presence_message(ra->client, buffer, size);
}

void zax_ra_list(zax_ra_t* ra, int category, zax_ra_visit_fn visit, void* context) {
  uint32_t bucket_index, index;
  rc_client_achievement_list_t* list = rc_client_create_achievement_list(
      ra->client, category, RC_CLIENT_ACHIEVEMENT_LIST_GROUPING_PROGRESS);
  if (!list)
    return;
  for (bucket_index = 0; bucket_index < list->num_buckets; bucket_index++) {
    const rc_client_achievement_bucket_t* bucket = &list->buckets[bucket_index];
    for (index = 0; index < bucket->num_achievements; index++) {
      const rc_client_achievement_t* achievement = bucket->achievements[index];
      zax_ra_entry_t entry;
      entry.bucket = bucket->bucket_type;
      entry.bucket_label = bucket->label;
      entry.id = achievement->id;
      entry.points = achievement->points;
      entry.title = achievement->title;
      entry.description = achievement->description;
      entry.progress = achievement->measured_progress;
      entry.percent = achievement->measured_percent;
      entry.unlocked = achievement->unlocked;
      entry.state = achievement->state;
      entry.type = achievement->type;
      entry.rarity = achievement->rarity;
      entry.badge_url = achievement->badge_url;
      entry.badge_locked_url = achievement->badge_locked_url;
      visit(context, &entry);
    }
  }
  rc_client_destroy_achievement_list(list);
}
