// aegis-highlight.js
hljs.registerLanguage('aegis', function(hljs) {
    return {
        name: 'Aegis',
        aliases: ['aeg'],
        keywords: {
            // Mots-clés de structure et contrôle
            keyword: 
                'var func class interface namespace import return ' +
                'if else while foreach in switch case default ' +
                'try catch throw new print input break continue ' +
                // Nouveaux mots-clés POO (v0.4+)
                'static final public private protected extends implements init',
            
            // Valeurs littérales
            literal: 'true false null',
            
            // Variables spéciales et Modules natifs
            built_in: 
                'this super ' + // Ajout de super
                'System File Http Json Math Random Test Assert Base64 Hash Date Process Path ' +
                'Socket Sqlite Bytes', // Nouveaux modules v0.4.3
            
            // Types
            type: 'int float string bool list dict func bytes void any'
        },
        contains: [
            // Chaînes de caractères avec interpolation ${...}
            {
                className: 'string',
                begin: '"', end: '"',
                contains: [
                    hljs.BACKSLASH_ESCAPE,
                    {
                        className: 'subst', // Pour colorer ${var}
                        begin: '\\$\\{', end: '\\}',
                        keywords: { keyword: 'this', built_in: 'this' }
                    }
                ]
            },
            hljs.C_LINE_COMMENT_MODE,     // // comment
            hljs.C_BLOCK_COMMENT_MODE,    // /* comment */
            hljs.NUMBER_MODE,             // 123, 12.5, 0xFF
            
            // Définition de fonctions
            {
                className: 'function',
                beginKeywords: 'func', 
                end: /\{/, 
                excludeEnd: true,
                contains: [
                    hljs.TITLE_MODE,
                    {
                        className: 'params',
                        begin: /\(/, end: /\)/,
                        // On permet de colorer les types dans les params (ex: data: dict)
                        keywords: { type: 'int float string bool list dict func bytes void any' }
                    }
                ]
            },
            
            // Définition de classes, interfaces et namespaces
            {
                className: 'class',
                beginKeywords: 'class interface namespace', 
                end: /\{/, 
                excludeEnd: true,
                contains: [
                    hljs.TITLE_MODE,
                    // Pour gérer "class User extends Model" ou "implements Interface"
                    {
                        beginKeywords: 'extends implements',
                        relevance: 0
                    }
                ]
            },
            
            // Décorateurs
            {
                className: 'meta',
                begin: '@[a-zA-Z_][a-zA-Z0-9_]*'
            }
        ]
    };
});

// Relance auto au chargement
hljs.initHighlightingOnLoad();
