WebSockets events documentation

**Version française dans websockets_en.md**

# Important
When connecting, it is required to send a Text event containing the Bearer token.

# Table of content
- [Events sent by client](#events-sent-by-client)
  - [Base structure](#base-structure)
  - [Actions](#actions)
    - [Subscribe to an event](#subscribe-to-an-event)
    - [Se désabonner d'un événement](#unsubscribe-from-an-event)
  - [Content](#content)
    - [Connected users count update](#connected-users-count-update)
- [Evénements envoyés par le serveur](#events-sent-by-server)
  - [Base structure](#base-structure-1)
  - [Events](#events)
    - [Connected users count update](#connected-users-count-update-1)
    - [Error](#error)

# Events sent by client
## Base structure
```json
{
  "action": String,
  "content": //content
}
```

## Actions
### Subscribe to an event
action = "subscribe_to_event"

### Unsubscribe from an event
action = "unsubscribe_from_event"

## Content
### Connected users count update
content = "connected_users_count_update"

Example :
```json
{
  "action": String,
  "content":"connected_users_count_update"
}
```

# Events sent by server
## Base structure
```json
{
  "event": String,
  "content": //content
}
```

## Events
### Connected users count update
event = "connected_users_count_update"

content: usize = connected user count

### Error
event = "error"

content: String = error message
