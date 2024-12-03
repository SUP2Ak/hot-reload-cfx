# FiveM Hot Reload (En Développement)

You can read this readme in English:

[![](https://img.shields.io/badge/English-000?style=for-the-badge&logo=github&logoColor=white)](README.md)

Une application de bureau multiplateforme construite avec Rust pour surveiller et recharger à chaud les ressources FiveM.

## Fonctionnalités Actuelles

### Système de Profils
- Gestion multi-profils de connexion
- Profil localhost par défaut (non supprimable) sans API key
- Profils distants avec authentification par API key
- Configuration simplifiée sans référencement du dossier resources

### Architecture
- Séparation claire UI (client) / Watcher (server)
- Générateur d'API key intégré
- Watcher autonome à placer à la racine du serveur
- Configuration automatique au premier lancement

### Communication
- WebSocket sécurisé pour les connexions distantes
- Authentification automatique selon le type de profil
- Détection en temps réel des changements

## En Développement

### Interface Utilisateur
- [ ] Système de checkbox pour ignorer/surveiller dossiers et fichiers
- [ ] Interface de logs (watcher, application, ressources)
- [ ] Amélioration de l'expérience utilisateur
- [ ] Gestion avancée des profils

### Watcher
- [ ] Finalisation du `handle_change`
- [ ] Gestion intelligente des modifications de fxmanifest
- [ ] Détection et traitement des ressources ajoutées/supprimées
- [ ] Optimisation des performances

### Ressource FiveM
- [ ] Amélioration de l'exécution des commandes internes
- [ ] Interface de logs détaillée
- [ ] Gestion des erreurs améliorée

## Installation

1. Téléchargez la dernière version
2. Pour le serveur : placez le watcher à la racine de votre serveur FiveM
3. Pour le client : lancez l'application UI
4. Configurez vos profils selon vos besoins

## Utilisation

1. Démarrez le watcher sur votre serveur
2. Lancez l'interface client
3. Sélectionnez ou créez un profil
4. Connectez-vous et commencez à développer

## Licence

Ce projet est sous licence MIT. Voir le fichier [LICENSE.txt](LICENSE.txt) pour plus de détails.

## Contributeurs

- [@sup2ak](https://github.com/sup2ak)

## Support

Pour signaler un bug ou proposer une amélioration, veuillez ouvrir une issue sur GitHub.