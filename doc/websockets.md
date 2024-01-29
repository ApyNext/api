Documentation des events WebSockets

**English version in the websockets_en.md file**

# Important
A la connexion, il est nécessaire d'envoyer un event de type Text, contenant le token Bearer.

# Sommaire
- [Evénements envoyés par le client](#evénements-envoyés-par-le-client)
  - [Structure de base](#structure-de-base)
  - [Actions](#actions)
    - [S'abonner à un événement](#sabonner-à-un-événement)
    - [Se désabonner d'un événement](#se-désabonner-dun-événement)
  - [Contenu](#contenu)
    - [Nombre d'utilisateurs connectés](#nombre-dutilisateurs-connectés)
- [Evénements envoyés par le serveur](#evénements-envoyés-par-le-serveur)
  - [Changement du nombre d'utilisateurs connectés](#changement-du-nombre-dutilisateurs-connectés)
  - [Nouveau post publié par un utilisateur suivi](#nouveau-post-publié-par-un-utilisateur-suivi)
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
## Changement du nombre d'utilisateurs connectés
```json
{
  "event": "connected_users_count_update",
  "content": <nombre> //nombre d'utilisateurs connectés
}
```

## Nouveau post publié par un utilisateur suivi
```json
{
  "event": "new_post_notification",
  "content": {
    "id": <nombre>, //id du post
    "title": <chaîne de caractères>, //titre du post
    "author": {
        "id": <nombre>, //id de l'auteur
        "username": <chaîne de caractères>, //nom d'utilisateur de l'auteur
        "permission": <nombre>, //permission de l'auteur : 0 = Utilisateur, 1 = Modérateur et 2 = Administrateur
    },
    "created_at": <timestamp UTC> //date de création du post
  }
}
```

## Erreur
```json
{
  "event": "error",
  "content": <chaîne de caractères> //message d'erreur
}
```
