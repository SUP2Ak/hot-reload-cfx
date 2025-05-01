# FiveM Hot Reload

You can read this readme in English:

[![](https://img.shields.io/badge/English-000?style=for-the-badge&logo=github&logoColor=white)](README.md)

Un outil de développement pour FiveM qui permet de recharger à chaud les ressources.
Cette Outil est composé en 3 parties :
- Le Watcher : Un exécutable qui se place à la racine de votre serveur FiveM (Cette racine où ce situe votre dossier `resources`).
- L'interface : Une application de bureau windows (stable) et macos | linux (sans doute dans le futur) qui permet de gérer les profils et de recevoir les notifications de changement de ressources.
- La Ressource : Une ressource FiveM qui permet de recevoir les notifications de changement de ressources et de les appliquer à chaud.

## Fonctionnalités Actuelles

### Interface
- Gestion multi-profils de connexion
- Profil localhost par défaut (non supprimable) sans API key
- Profils distants avec authentification par API key
- Configuration simplifiée sans référencement du dossier resources
- Arborescence des ressources fetchée automatiquement ainsi que les fichiers watchés

### Watcher
- Détection en temps réel des changements
- Application des changements à chaud
- Gestion intelligente des modifications de fxmanifest ainsi que des fichier supprimés/ajoutés (Afin de refresh avant de relancer la ressource)
- Gestion intelligente des fichiers uniquement modifiés (Afin de relancer sans refresh)
- Intégration d'un CLI pour les commandes internes (help, version, etc...)

### Ressource
- Permet d'appliquer les changements à chaud détectés par le watcher
- requiert les permissions dans le server.cfg
```
add_ace resource.hot-reload command.start allow
add_ace resource.hot-reload command.stop allow
add_ace resource.hot-reload command.ensure allow
add_ace resource.hot-reload command.refresh allow
add_ace resource.hot-reload command.add_ace allow
add_ace resource.hot-reload command.add_principal allow
```

## Licence

Ce projet est sous licence MIT. Voir le fichier [LICENSE.txt](LICENSE.txt) pour plus de détails.

## Contributeurs

Vous voulez contribuer à ce projet? Cliquez sur le badge ci-dessous.

[![](https://img.shields.io/badge/-Contribution-000?style=for-the-badge&logo=github&logoColor=white)](CONTRIBUTING.fr.md)

- [@sup2ak](https://github.com/sup2ak)

## Support

Pour signaler un bug ou proposer une amélioration, veuillez ouvrir une issue sur GitHub.