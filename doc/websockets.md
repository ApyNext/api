Documentation des events WebSockets

**English version in the websockets_en.md file**

# Sommaire
- [Evénements envoyés par le client](#evénements-envoyés-par-le-client)
  - [Structure de base](#structure-de-base)
  - [Actions](#actions)
    - [S'abonner à un événement](#sabonner-à-un-événement)
    - [Se désabonner d'un événement](#se-désabonner-dun-événement)
  - [Contenu](#contenu)
    - [Nombre d'utilisateurs connectés](#nombre-dutilisateurs-connectés)
- [Evénements envoyés par le serveur](#evénements-envoyés-par-le-serveur)
  - [Structure de base](#structure-de-base-1)
  - [Evénements](#evénements)
    - [Changement du nombre d'utilisateurs connectés](#changement-du-nombre-dutilisateurs-connectés)
    - [Erreur](#erreur)

# Evénements envoyés par le client
## Structure de base
```json
{
  "action": String,
  "content": //contenu
}
```

## Actions
### S'abonner à un événement
action = "subscribe_to_event"

### Se désabonner d'un événement
action = "unsubscribe_from_event"

## Contenu
### Nombre d'utilisateurs connectés
content = "connected_users_count_update"

Exemple :
```json
{
  "action": String,
  "content":"connected_users_count_update"
}
```

# Evénements envoyés par le serveur
## Structure de base
```json
{
  "event": String,
  "content": //contenu
}
```

## Evénements
### Changement du nombre d'utilisateurs connectés
event = "connected_users_count_update"
content: usize = nombre d'utilisateurs connectés

### Erreur
event = "error"
content: String = message d'erreur
