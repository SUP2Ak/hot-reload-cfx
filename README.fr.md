# FiveM Hot Reload (En Développement)

You can read this readme in English:

[![](https://img.shields.io/badge/English-000?style=for-the-badge&logo=github&logoColor=white)](README.md)

Une application de bureau multiplateforme construite avec Rust pour surveiller et recharger à chaud les ressources FiveM.

## Fonctionnalités (En Développement)

- 🔄 Surveillance des ressources en temps réel
- 🚀 Rechargement automatique à chaud
- 📁 Visualisation de l'arborescence des ressources
- 🌐 Communication WebSocket
- 💻 Support multiplateforme (Windows, Linux, MacOS)
- 🎨 Interface moderne avec egui

## Installation (Uniquement lorsqu'une version sera disponible)

1. Téléchargez la dernière version pour votre système d'exploitation :
   - Windows : hot-reload.exe
   - Linux : hot-reload
   - MacOS : hot-reload.app

Ou compilez depuis les sources :

1. Assurez-vous d'avoir Rust installé
2. Clonez ce dépôt
3. Exécutez : `cargo build --release`

## Utilisation

1. Lancez l'application
2. Cliquez sur "📂 Sélectionner les Ressources" pour choisir votre dossier de ressources FiveM
3. L'application va automatiquement :
   - Scanner les ressources
   - Afficher l'arborescence des ressources
   - Surveiller les changements dans les fichiers .lua et .js
   - Recharger à chaud les ressources modifiées

## Configuration

L'application crée un fichier server_config.json pour stocker :

- Le chemin du dossier des ressources
- Les paramètres de connexion WebSocket

## Détails Techniques

- Construit avec Rust et eframe/egui
- Utilise tokio pour les opérations asynchrones
- Communication WebSocket pour le rechargement à chaud
- Surveillance du système de fichiers avec notify
- Supporte les fichiers .lua et .js et peut-être plus tard .net.dll

## À Faire

- [ ] Améliorer la gestion des erreurs
- [ ] Ajouter la sélection/désélection des ressources
- [ ] Personnaliser les paramètres de connexion WebSocket
- [ ] Revoir la gestion des événements
- [ ] Ajouter une interface de journalisation
- [ ] Séparer en service API distinct

## Licence

Ce projet est sous licence MIT. Voir le fichier [LICENSE.txt](LICENSE.txt) pour plus de détails.

## Contributeurs

- [@sup2ak](https://github.com/sup2ak)

## Problèmes

Si vous rencontrez des problèmes ou avez des suggestions d'amélioration, veuillez ouvrir une issue sur le [dépôt GitHub](https://github.com/sup2ak/fivem-hot-reload/issues).

## Pull Requests

Nous accueillons les contributions pour améliorer le projet. Veuillez consulter notre [CONTRIBUTING.md](CONTRIBUTING.fr.md) pour les directives sur la soumission d'améliorations et de corrections de bugs.