//! Проверки рецептов пресета: битый рецепт не должен доезжать до игры.

use std::path::Path;

use crate::config::GameConfig;
use crate::craft::{self, CraftContext};
use crate::item::{Inventory, Item};

fn core_config() -> GameConfig {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game/presets/core");
    GameConfig::load_from(&base.to_string_lossy())
}

#[test]
fn preset_recipes_reference_real_items() {
    let cfg = core_config();
    assert!(!cfg.recipes.is_empty(), "в core-пресете нет ни одного рецепта");

    for recipe in &cfg.recipes {
        assert!(
            cfg.items.iter().any(|item| item.id == recipe.output),
            "рецепт {}: результат '{}' отсутствует в items.json",
            recipe.id,
            recipe.output
        );
        assert!(!recipe.inputs.is_empty(), "рецепт {}: нет ингредиентов", recipe.id);
        for input in &recipe.inputs {
            assert!(
                cfg.items.iter().any(|item| item.id == input.item),
                "рецепт {}: ингредиент '{}' отсутствует в items.json",
                recipe.id,
                input.item
            );
            assert!(input.qty > 0, "рецепт {}: нулевое количество", recipe.id);
        }
        assert!(
            recipe.inputs.iter().all(|input| input.item != recipe.output),
            "рецепт {} превращает предмет сам в себя",
            recipe.id
        );
        assert!(!recipe.name_ru.is_empty(), "рецепт {}: нет названия", recipe.id);
    }
}

#[test]
fn recipe_ids_are_unique() {
    let cfg = core_config();
    let mut seen = std::collections::HashSet::new();
    for recipe in &cfg.recipes {
        assert!(seen.insert(&recipe.id), "рецепт {} объявлен дважды", recipe.id);
    }
}

/// Сквозная проверка на настоящих данных: из хлеба и осколка получается аптечка.
#[test]
fn field_medkit_works_on_real_preset() {
    let cfg = core_config();
    let recipe = cfg.recipe("field_medkit").expect("рецепт аптечки на месте");

    let mut inventory = Inventory::default();
    inventory.add(Item::new("bread", "Хлеб", "", 2));
    inventory.add(Item::new("neon_shard", "Осколок", "", 1));

    let ctx = CraftContext::default();
    let made = craft::craft(&mut inventory, recipe, &ctx, "Аптечка", "").unwrap();

    assert_eq!(made.id, "medkit");
    assert_eq!(inventory.count("medkit"), 1);
    assert_eq!(inventory.count("bread"), 0);
}

/// Рецепты со станцией не должны крафтиться в чистом поле.
#[test]
fn bench_recipes_need_a_bench() {
    let cfg = core_config();
    let bench: Vec<_> = cfg.recipes.iter().filter(|r| r.station == "bench").collect();
    assert!(!bench.is_empty(), "ожидались рецепты верстака");

    let mut inventory = Inventory::default();
    for recipe in &bench {
        for input in &recipe.inputs {
            inventory.add(Item::new(&input.item, &input.item, "", input.qty));
        }
    }

    let nowhere = CraftContext::default();
    for recipe in &bench {
        assert!(
            craft::check(&inventory, recipe, &nowhere).is_err(),
            "рецепт {} крафтится без верстака",
            recipe.id
        );
    }
}
