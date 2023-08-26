# api
L'API officielle de ApyNext

# Configuration
- Configurez Postgres sur votre machine, vous pouvez l'installer directement (plus d'infos [ici](https://www.postgresql.org/docs/15/install-short.html)) - choisissez également un mot de passe pour l'utilisateur postgres de la base de données - ou vous pouvez juste utiliser la Dockerfile de ce projet :
1) Installez Docker sur votre machine (plus d'informations [ici](https://www.docker.com/)).
2) Exécutez cette commande en remplace `<mot de passe>` par le mot de passe que vous souhaitez pour la BDD :
```bash
docker build -t postgres . && docker run -e POSTGRES_PASSWORD="<mot de passe>" -p 5432:5432 postgres
```
- Installez la CLI de Shuttle, plus d'infos [ici](https://docs.shuttle.rs/introduction/installation).
- Et la CLI de SQLx, plus d'infos [ici](https://docs.rs/crate/sqlx-cli/latest).
- Renommez (ou copiez) les fichiers Secrets.toml.example en Secrets.toml et renseignez les informations manquantes

# Lancer l'API
Pour lancer l'API localement, il suffit d'exécuter les commandes
```bash
cargo sqlx prepare --database-url <l'URL de la BDD>
```
et
```bash
cargo sqlx migrate run
```
et
```bash
cargo shuttle run
```
Si vous voulez déployer l'API, vous pouvez vous [créer un compte Shuttle](https://console.shuttle.rs/login) puis suivre les indications disponibles [ici](https://console.shuttle.rs/new-project) (sauf l'installation de la CLI, car vous l'avez déjà fait).

# Documentation
## Gestion de compte
### Créer un compte
Requête : `POST /register`

Body (JSON) :
- username => chaîne de caractères entre 5 et 12 caractères compris, commençant par une lettre et ne pouvant contenir que des lettres, des nombres et des underscores
- email => chaîne de caractères d'un email valide
- password => chaîne de caractères contenant au moins 8 caractères
- birthdate => timestamp Unix entre 1900 et aujourd'hui
- biography => chaîne de caractères d'une longueur maximale de 300 caractères
- is_male (facultatif pour des raisons de confidentialité) => booléen (true pour un homme et false pour une femme)

Renvoie :
- Code de status `200 Ok`
- Code de status `403 Forbidden` et le message d'erreur lors d'une erreur client
- Code de status `500 Internal Server Error` lors d'une erreur serveur

### Vérifier l'email
Requête : `POST /register/email_confirm`

Query :
- token : chaîne de caractères représentant un JWT de vérification d'email

Renvoie :
- Code de status `200 Ok` et un JWT de connexion
- Code de status `401 Unauthorized` quand le lien est invalide
- Code de status `403 Forbidden` quand le lien est expiré
- Code de status `500 Internal Server Error` lors d'une erreur serveur