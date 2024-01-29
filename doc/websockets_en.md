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
  - [Connected users count update](#connected-users-count-update-1)
  - [New post published by an user followed](#new-post-published-by-an-user-followed)
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
## Connected users count update
```json
{
  "event": "connected_users_count_update",
  "content": <number> //number of users connected
}
```

## New post published by an user followed
```json
{
  "event": "new_post_notification",
  "content": {
    "id": <number>, //post id
    "title": <string>, //post title
    "author": {
        "id": <number>, //author id
        "username": <string>, //author username
        "permission": <number>, //author permission : 0 = User, 1 = Moderator and 2 = Administrator
    },
    "created_at": <UTC timestamp> //post creation date
  }
}
```

## Error
```json
{
  "event": "error",
  "content": <string> //error message
}
```