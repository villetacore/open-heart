//! Крафт: превращение предметов в другие предметы по рецептам.
//!
//! Правила лежат здесь, в ядре, потому что в сетевой игре крафт обязан
//! проверять сервер — иначе клиент «скрафтит» что угодно из ничего
//! (docs/MULTIPLAYER.md §5). Сами рецепты — данные пресета (`recipes.json`),
//! добавление нового не требует ни строчки кода.

use crate::config::RecipeCfg;
use crate::item::{Inventory, Item};

/// Почему крафт не состоялся.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CraftError {
    /// Рецепта с таким id нет в пресете.
    UnknownRecipe,
    /// Не хватает ингредиентов: id и сколько ещё нужно.
    Missing { item: String, need: u32 },
    /// Крафт требует станции (верстак), а игрок не рядом с ней.
    StationRequired(String),
    /// Не открыт: нужен флаг прогресса (квест, чертёж).
    Locked(String),
}

impl CraftError {
    /// Короткое человеческое объяснение — его показывает клиент.
    pub fn message(&self, lang: &str) -> String {
        let en = lang == "en";
        match self {
            Self::UnknownRecipe => {
                if en { "Unknown recipe".into() } else { "Неизвестный рецепт".into() }
            }
            Self::Missing { item, need } => {
                if en {
                    format!("Not enough: {item} ×{need}")
                } else {
                    format!("Не хватает: {item} ×{need}")
                }
            }
            Self::StationRequired(station) => {
                if en {
                    format!("Requires station: {station}")
                } else {
                    format!("Нужна станция: {station}")
                }
            }
            Self::Locked(flag) => {
                if en {
                    format!("Recipe locked ({flag})")
                } else {
                    format!("Рецепт ещё не открыт ({flag})")
                }
            }
        }
    }
}

/// Где игрок находится с точки зрения крафта.
#[derive(Clone, Debug, Default)]
pub struct CraftContext<'a> {
    /// Станции рядом: пусто — крафт «на коленке».
    pub stations: &'a [String],
    /// Флаги прогресса игрока (открытые чертежи, выполненные квесты).
    pub flags: &'a [String],
}

/// Хватает ли всего для рецепта. Ничего не меняет.
pub fn check(inventory: &Inventory, recipe: &RecipeCfg, ctx: &CraftContext) -> Result<(), CraftError> {
    if !recipe.station.is_empty() && !ctx.stations.contains(&recipe.station) {
        return Err(CraftError::StationRequired(recipe.station.clone()));
    }
    if let Some(flag) = recipe.requires_flag.as_ref() {
        if !ctx.flags.contains(flag) {
            return Err(CraftError::Locked(flag.clone()));
        }
    }
    for input in &recipe.inputs {
        let have = inventory.count(&input.item);
        if have < input.qty {
            return Err(CraftError::Missing {
                item: input.item.clone(),
                need: input.qty - have,
            });
        }
    }
    Ok(())
}

/// Выполнить рецепт: списать ингредиенты, выдать результат.
///
/// `name` и `description` берутся из данных пресета вызывающим — ядро не знает
/// про язык интерфейса.
pub fn craft(
    inventory: &mut Inventory,
    recipe: &RecipeCfg,
    ctx: &CraftContext,
    name: &str,
    description: &str,
) -> Result<Item, CraftError> {
    check(inventory, recipe, ctx)?;
    for input in &recipe.inputs {
        // check() уже подтвердил количество, поэтому списание не может провалиться.
        inventory.remove(&input.item, input.qty);
    }
    let item = Item::new(&recipe.output, name, description, recipe.output_qty.max(1));
    inventory.add(item.clone());
    Ok(item)
}

/// Рецепты, которые прямо сейчас можно выполнить.
pub fn available<'a>(
    inventory: &Inventory,
    recipes: &'a [RecipeCfg],
    ctx: &CraftContext,
) -> Vec<&'a RecipeCfg> {
    recipes
        .iter()
        .filter(|recipe| check(inventory, recipe, ctx).is_ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RecipeInput;

    fn recipe() -> RecipeCfg {
        RecipeCfg {
            id: "medkit_from_scraps".into(),
            output: "medkit".into(),
            output_qty: 1,
            inputs: vec![
                RecipeInput { item: "bread".into(), qty: 2 },
                RecipeInput { item: "neon_shard".into(), qty: 1 },
            ],
            station: String::new(),
            requires_flag: None,
            time: 1.5,
            name_ru: "Аптечка из подручного".into(),
            name_en: "Field medkit".into(),
        }
    }

    fn inventory(items: &[(&str, u32)]) -> Inventory {
        let mut inventory = Inventory::default();
        for (id, qty) in items {
            inventory.add(Item::new(id, id, "", *qty));
        }
        inventory
    }

    #[test]
    fn crafts_and_consumes_inputs() {
        let mut inv = inventory(&[("bread", 3), ("neon_shard", 1)]);
        let ctx = CraftContext::default();

        let made = craft(&mut inv, &recipe(), &ctx, "Аптечка", "").unwrap();
        assert_eq!(made.id, "medkit");
        assert_eq!(inv.count("medkit"), 1);
        assert_eq!(inv.count("bread"), 1, "должно списаться ровно два хлеба");
        assert_eq!(inv.count("neon_shard"), 0);
    }

    #[test]
    fn refuses_without_ingredients() {
        let mut inv = inventory(&[("bread", 1), ("neon_shard", 1)]);
        let ctx = CraftContext::default();

        let error = craft(&mut inv, &recipe(), &ctx, "Аптечка", "").unwrap_err();
        assert_eq!(error, CraftError::Missing { item: "bread".into(), need: 1 });
        // Провалившийся крафт не должен ничего съесть.
        assert_eq!(inv.count("bread"), 1);
        assert_eq!(inv.count("neon_shard"), 1);
        assert_eq!(inv.count("medkit"), 0);
    }

    #[test]
    fn station_and_flags_gate_the_recipe() {
        let mut advanced = recipe();
        advanced.station = "bench".into();
        advanced.requires_flag = Some("blueprint_medkit".into());

        let inv = inventory(&[("bread", 2), ("neon_shard", 1)]);

        let nowhere = CraftContext::default();
        assert_eq!(
            check(&inv, &advanced, &nowhere),
            Err(CraftError::StationRequired("bench".into()))
        );

        let stations = ["bench".to_string()];
        let at_bench = CraftContext { stations: &stations, flags: &[] };
        assert_eq!(
            check(&inv, &advanced, &at_bench),
            Err(CraftError::Locked("blueprint_medkit".into()))
        );

        let flags = ["blueprint_medkit".to_string()];
        let ready = CraftContext { stations: &stations, flags: &flags };
        assert!(check(&inv, &advanced, &ready).is_ok());
    }

    #[test]
    fn available_lists_only_doable() {
        let inv = inventory(&[("bread", 2), ("neon_shard", 1)]);
        let mut locked = recipe();
        locked.id = "locked".into();
        locked.station = "forge".into();

        let recipes = vec![recipe(), locked];
        let ctx = CraftContext::default();
        let list = available(&inv, &recipes, &ctx);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "medkit_from_scraps");
    }
}
