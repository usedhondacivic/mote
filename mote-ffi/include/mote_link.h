#ifndef MOTE_LINK_H
#define MOTE_LINK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Opaque handle for a Mote communication link.
 */
typedef struct MoteLinkHandle MoteLinkHandle;

/**
 * Create a new MoteLink handle.
 *
 * The returned pointer must be freed with mote_link_free().
 */
MoteLinkHandle *mote_link_new(void);

/**
 * Free a MoteLink handle.
 */
void mote_link_free(MoteLinkHandle *handle);

/**
 * Queue a JSON-encoded host-to-mote message for transmission.
 *
 * @param handle       Valid MoteLink handle.
 * @param json_message Null-terminated JSON string representing the message.
 *                     Examples: "\"Ping\"", "{\"SetUID\":{\"uid\":\"mote-abc\"}}"
 * @return             0 on success, -1 on error.
 */
int mote_link_send(MoteLinkHandle *handle, const char *json_message);

/**
 * Copy the next transmit packet into buf.
 *
 * Packets must be sent over the network in the order they are returned.
 * Call repeatedly until 0 is returned to drain all pending packets.
 *
 * @param handle   Valid MoteLink handle.
 * @param buf      Output buffer to write packet bytes into.
 * @param buf_len  Size of buf in bytes. 4096 is sufficient for all packet types.
 * @return         Number of bytes written, 0 if no packet is pending, -1 if buf is too small.
 */
int mote_link_poll_transmit(MoteLinkHandle *handle, uint8_t *buf, int buf_len);

/**
 * Feed a received packet into the link.
 *
 * @param handle   Valid MoteLink handle.
 * @param buf      Received packet bytes.
 * @param buf_len  Number of bytes in buf.
 * @return         0 on success.
 */
int mote_link_handle_receive(MoteLinkHandle *handle, const uint8_t *buf, int buf_len);

/**
 * Copy the next decoded mote-to-host message as a null-terminated JSON string into buf.
 *
 * Call repeatedly until 0 is returned to drain all pending messages.
 *
 * @param handle   Valid MoteLink handle.
 * @param buf      Output buffer for the null-terminated JSON string.
 * @param buf_len  Size of buf in bytes. 65536 is sufficient for all message types.
 * @return         Number of bytes written including the null terminator,
 *                 0 if no message is ready, -1 on error or if buf is too small.
 */
int mote_link_poll_receive(MoteLinkHandle *handle, char *buf, int buf_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* MOTE_LINK_H */
