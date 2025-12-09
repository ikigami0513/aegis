import random
import time

print("--- PYTHON RPG BATTLE SIMULATOR ---")

# 1. Configuration (Namespace simulé par classe)
class Config:
    MAX_ROUNDS = 50
    CRIT_CHANCE = 20

# 2. CLASSE
class Entity:
    def __init__(self, name, hp, atk, defender_def):
        self.name = name
        self.hp = hp
        self.atk = atk
        self.defender_def = defender_def

    def is_alive(self):
        return self.hp > 0

    def take_damage(self, amount):
        if amount < 0:
            print("  [BUG] Degats negatifs detectes !")
            return 0
        else:
            real_dmg = amount - self.defender_def
            if real_dmg < 1: real_dmg = 1
            self.hp = self.hp - real_dmg
            return real_dmg

    def attack(self, target):
        if not target.is_alive():
            raise Exception("Impossible d'attaquer une cible morte !")

        dmg = self.atk
        # Simulation critique
        roll = random.randint(0, 100)
        if roll < Config.CRIT_CHANCE:
            dmg = dmg * 2
            print("  >>> CRITIQUE ! <<<")
        
        inflicted = target.take_damage(dmg)
        print(f"{self.name} attaque {target.name} pour {inflicted} degats.")

class Hero(Entity):
    def __init__(self, name, hp, atk, defender_def):
        super().__init__(name, hp, atk, defender_def)
        self.inventory = {
            "potions": 2,
            "bombs": 1
        }

    def use_potion(self):
        count = self.inventory.get("potions")
        if count > 0:
            self.hp = self.hp + 50
            self.inventory["potions"] = count - 1
            print(f"{self.name} utilise une potion (+50 HP).")
        else:
            raise Exception("Inventaire vide")

# --- SETUP ---
start_time = time.time()

hero = Hero("Arthur", 200, 25, 5)
monsters = []
for i in range(5):
    monsters.append(Entity(f"Gobelin_{i}", 60, 10, 2))

print(f"Debut du combat : {hero.name} vs {len(monsters)} monstres.")

# --- GAME LOOP ---
round_num = 0
battle_over = False

while not battle_over and round_num < Config.MAX_ROUNDS:
    round_num += 1
    print(f"\n--- ROUND {round_num} ---")

    # 4. FONCTIONNEL (Filter)
    living_monsters = list(filter(lambda m: m.is_alive(), monsters))

    if len(living_monsters) == 0:
        print("VICTOIRE ! Tous les monstres sont vaincus.")
        battle_over = True
    else:
        # Tour du Héros
        # Map pour stats (inutilisé mais pour test)
        stats = list(map(lambda m: m.hp, living_monsters))

        try:
            if hero.hp < 50:
                hero.use_potion()
            else:
                target = living_monsters[0]
                hero.attack(target)
        except Exception as e:
            print(f"[Erreur Hero] {e}")
            target = living_monsters[0]
            hero.attack(target)

        # Tour des Monstres
        for m in living_monsters:
            if hero.is_alive():
                m.attack(hero)

        if not hero.is_alive():
            print(f"DEFAITE... {hero.name} est mort.")
            battle_over = True

end_time = time.time()
print("\nFin de la simulation.")
print(f"Temps d'exécution Python : {(end_time - start_time) * 1000:.2f}ms")
