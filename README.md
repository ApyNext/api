# api
L'API officielle de ApyNext

**English version in the README_en.md file**

# Sommaire
- [Configuration](#configuration)
- [Lancer l'API](#lancer-lapi)
- [Documentation](#documentation)
    - [Gestion de compte](#gestion-de-compte)
        - [Créer un compte](#créer-un-compte)
        - [Vérifier l'email (lien envoyé par email)](#vérifier-lemail-lien-envoyé-par-email)
        - [Se connecter](#se-connecter)
        - [A2F (lien envoyé par email)](#a2f-lien-envoyé-par-email)

# Configuration
- Configurez Postgres sur votre machine, vous pouvez l'installer directement (plus d'infos [ici](https://www.postgresql.org/docs/15/install-short.html)) - choisissez également un mot de passe pour l'utilisateur postgres de la base de données - ou vous pouvez juste utiliser le fichier docker-compose.yml de ce projet :
1) Installez Docker sur votre machine (plus d'informations [ici](https://www.docker.com/)).
2) Installez également Docker compose (plus d'informations [ici](https://docs.docker.com/compose/install/)).
3) Exécutez cette commande en remplaçant `<mot de passe>` par le mot de passe que vous souhaitez pour la BDD :
```bash
POSTGRES_PASSWORD=`<mot de passe>` docker compose up -d
```
- Renommez (ou copiez) le fichier .env.example en .env et renseignez les informations manquantes

# Lancer l'API
Pour lancer l'API localement, il suffit d'exécuter la commande
```bash
cargo run
```

# Documentation

## Route de test

Requête : `GET /`

Renvoie :
- Code de status 200 et le message "Ok"

## Gestion de compte
### Créer un compte
Requête : `POST /register`

Body (JSON) :
- username => chaîne de caractères entre 5 et 12 caractères compris, commençant par une lettre et ne pouvant contenir que des lettres, des nombres et des underscores
- email => chaîne de caractères d'un email valide
- password => chaîne de caractères contenant au moins 8 caractères
- birthdate => timestamp Unix entre 1900 et aujourd'hui
- is_male (facultatif pour des raisons de confidentialité) => booléen (true pour un homme et false pour une femme)

Renvoie :
- Code de status `200 Ok`
- Code de status `400 Bad request` quand le body n'est pas un JSON valide
- Code de status `403 Forbidden` et le message d'erreur lors d'une erreur client
- Code de status `415 Unsupported Media Type` quand le header `Content-Type: application/json` est manquant
- Code de status `422 Unprocessable Entity` lorsqu'un field JSON est manquant
- Code de status `500 Internal Server Error` lors d'une erreur serveur

### Vérifier l'email (lien envoyé par email)
Requête : `POST /register/email_confirm`

Body (chaîne de caractères) :
- token de confirmation d'email

Renvoie :
- Code de status `200 Ok` et un token de connexion stocké comme cookie
- Code de status `403 Forbidden` et le message d'erreur quand le token est manquant, invalide ou expiré par exemple
- Code de status `500 Internal Server Error` lors d'une erreur serveur

### Se connecter
Requête : `POST /login`

Body (JSON) :
- username_or_email => chaîne de caractères représentant soit :
    - un pseudo entre 5 et 12 caractères compris, commençant par une lettre et ne pouvant contenir que des lettres, des nombres et des underscores
    - un email valide
- password => mot de passe (au moins 8 caractères)

Renvoie :
- Code de status `200 Ok`
- Code de status `400 Bad request` quand le body n'est pas un JSON valide
- Code de status `403 Forbidden` et le message d'erreur lors d'une erreur client
- Code de status `415 Unsupported Media Type` quand le header `Content-Type: application/json` est manquant
- Code de status `422 Unprocessable Entity` lorsqu'un field JSON est manquant
- Code de status `500 Internal Server Error` lors d'une erreur serveur

### A2F (lien envoyé par email)
Requête : `POST /login/a2f`

Body (châine de caractères) :
- token de vérification de connexion

Renvoie :
- Code de status `200 Ok` et un token de connexion stocké comme cookie
- Code de status `403 Forbidden` et le message d'erreur quand le token est manquant, invalide ou expiré

## WebSockets
Requête : `GET /ws`

Headers :
- Token Bearer (facultatif)

## Suivre un utilisateur
Requête : `POST /:id/follow`

Headers :
- Token Bearer

Renvoie :
- Code de status `200 Ok`
- Code de status `403 Forbidden` avec le message d'erreur lors d'une erreur client
- Code de status `500 Internal Server Error` lors d'une erreur serveur
