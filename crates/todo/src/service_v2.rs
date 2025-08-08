use crate::graph::{DependencyGraphV2, GraphStats};
use crate::store_v2::TodoStoreV2;
use crate::types::*;
use anyhow::Result;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Оптимизированный TodoService с кэшированием и событиями
pub struct TodoServiceV2 {
    // Хранилище с пулом соединений
    store: Arc<TodoStoreV2>,
    // Граф зависимостей с конкурентным доступом
    graph: Arc<DependencyGraphV2>,
    // LRU кэш для часто запрашиваемых задач
    cache: Arc<Mutex<LruCache<Uuid, TodoItem>>>,
    // Кэш для готовых задач
    ready_cache: Arc<DashMap<(), Vec<Uuid>>>,
    // Канал событий для реактивности
    events_tx: mpsc::UnboundedSender<TodoEvent>,
    events_rx: Arc<Mutex<mpsc::UnboundedReceiver<TodoEvent>>>,
}

impl TodoServiceV2 {
    /// Создать новый сервис с оптимизациями
    pub async fn new<P: AsRef<Path>>(
        db_path: P,
        pool_size: u32,
        cache_size: usize,
    ) -> Result<Self> {
        let store = Arc::new(TodoStoreV2::new(db_path, pool_size).await?);
        let graph = Arc::new(DependencyGraphV2::new());

        // Загружаем активные задачи в граф
        let active_tasks = store.get_stats().await?;
        if active_tasks.total > 0 {
            // Загружаем все активные задачи батчем
            let all_tasks = store.search("", active_tasks.total).await?;
            graph.load_from_tasks(all_tasks)?;
        }

        // Создаем канал событий
        let (events_tx, events_rx) = mpsc::unbounded_channel();

        info!(
            "TodoServiceV2 initialized with {} active tasks",
            active_tasks.total
        );

        Ok(Self {
            store,
            graph,
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(cache_size).unwrap(),
            ))),
            ready_cache: Arc::new(DashMap::new()),
            events_tx,
            events_rx: Arc::new(Mutex::new(events_rx)),
        })
    }

    /// Создать задачу с умной валидацией
    #[instrument(skip(self))]
    pub async fn create_task(
        &self,
        title: String,
        description: String,
        priority: Priority,
        tags: Vec<String>,
    ) -> Result<TodoItem> {
        let task = TodoItem {
            title,
            description,
            state: TaskState::Ready,
            priority,
            tags,
            ..Default::default()
        };

        let created = self.store.create(task).await?;
        self.graph.upsert_task(&created)?;

        // Инвалидируем кэш готовых задач
        self.ready_cache.clear();

        // Публикуем событие
        self.emit_event(TodoEvent::TaskCreated {
            task_id: created.id,
            title: created.title.clone(),
            auto_generated: created.auto_generated,
        });

        debug!("Created task: {} ({})", created.title, created.id);
        Ok(created)
    }

    /// Создать подзадачи батчем
    pub async fn create_subtasks(
        &self,
        parent_id: &Uuid,
        subtasks: Vec<(String, String)>,
    ) -> Result<Vec<TodoItem>> {
        let parent = self
            .get_cached(parent_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Parent task not found"))?;

        let mut created_tasks: Vec<TodoItem> = Vec::new();

        for (title, description) in subtasks {
            let mut task = TodoItem {
                title,
                description,
                parent_id: Some(*parent_id),
                state: TaskState::Blocked, // Подзадачи начинают заблокированными
                priority: parent.priority,
                auto_generated: true,
                ..Default::default()
            };

            // Добавляем зависимость от всех предыдущих подзадач
            if let Some(last_task) = created_tasks.last() {
                task.depends_on.push(last_task.id);
            }

            let created = self.store.create(task).await?;
            self.graph.upsert_task(&created)?;
            created_tasks.push(created);
        }

        // Инвалидируем кэши
        self.ready_cache.clear();

        info!("Created {} subtasks for {}", created_tasks.len(), parent_id);
        Ok(created_tasks)
    }

    /// Получить задачу с кэшированием
    pub async fn get_cached(&self, id: &Uuid) -> Result<Option<TodoItem>> {
        // Проверяем кэш
        {
            let mut cache = self.cache.lock();
            if let Some(task) = cache.get(id) {
                return Ok(Some(task.clone()));
            }
        }

        // Загружаем из БД
        if let Some(task) = self.store.get(id).await? {
            // Добавляем в кэш
            let mut cache = self.cache.lock();
            cache.put(*id, task.clone());
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Получить следующие готовые задачи (с кэшированием)
    #[instrument(skip(self))]
    pub async fn get_next_ready(&self, count: usize) -> Result<Vec<TodoItem>> {
        // Проверяем кэш готовых задач
        if let Some(cached_ids) = self.ready_cache.get(&()) {
            if !cached_ids.is_empty() {
                let tasks = self
                    .store
                    .get_batch(&cached_ids[..count.min(cached_ids.len())])
                    .await?;
                if !tasks.is_empty() {
                    return Ok(tasks);
                }
            }
        }

        // Используем оптимизированный запрос
        let ready_tasks = self.store.find_ready_tasks(count * 2).await?;

        // Кэшируем ID готовых задач
        let ready_ids: Vec<Uuid> = ready_tasks.iter().map(|t| t.id).collect();
        self.ready_cache.insert((), ready_ids);

        // Добавляем в LRU кэш
        {
            let mut cache = self.cache.lock();
            for task in &ready_tasks {
                cache.put(task.id, task.clone());
            }
        }

        Ok(ready_tasks.into_iter().take(count).collect())
    }

    /// Обновить состояние с оптимизированным каскадом
    #[instrument(skip(self))]
    pub async fn update_state(&self, id: &Uuid, new_state: TaskState) -> Result<()> {
        let old_task = self
            .get_cached(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let old_state = old_task.state;

        // Обновляем в БД с каскадом
        let affected_ids = self.store.update_state_cascade(id, new_state).await?;

        // Обновляем граф для всех затронутых задач
        for affected_id in &affected_ids {
            self.graph.update_state(
                affected_id,
                if affected_id == id {
                    new_state
                } else {
                    TaskState::Ready
                },
            )?;

            // Инвалидируем кэш
            self.cache.lock().pop(affected_id);
        }

        // Инвалидируем кэш готовых задач
        self.ready_cache.clear();

        // Публикуем события
        self.emit_event(TodoEvent::StateChanged {
            task_id: *id,
            old_state,
            new_state,
            timestamp: chrono::Utc::now(),
        });

        // Если задача выполнена, публикуем дополнительное событие
        if new_state == TaskState::Done {
            if let Some(duration) = old_task.started_at.map(|start| chrono::Utc::now() - start) {
                self.emit_event(TodoEvent::TaskCompleted {
                    task_id: *id,
                    duration,
                    artifacts: old_task.artifacts.clone(),
                });
            }
        }

        info!(
            "Updated task {} state: {:?} -> {:?}, affected {} tasks",
            id,
            old_state,
            new_state,
            affected_ids.len()
        );

        Ok(())
    }

    /// Добавить зависимость с проверкой циклов
    pub async fn add_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        // Проверяем на циклы
        if self.graph.would_create_cycle(depends_on, task_id)? {
            return Err(anyhow::anyhow!("Dependency would create a cycle"));
        }

        // Добавляем в БД
        self.store
            .get(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        // Добавляем зависимость в базу данных
        self.store.add_dependency(task_id, depends_on).await?;

        // Обновляем граф
        if let Some(mut task) = self.get_cached(task_id).await? {
            task.depends_on.push(*depends_on);
            self.graph.upsert_task(&task)?;

            // Проверяем нужно ли изменить состояние
            if task.state == TaskState::Ready && !self.graph.is_ready(task_id)? {
                self.update_state(task_id, TaskState::Blocked).await?;
            }
        }

        // Инвалидируем кэши
        self.cache.lock().pop(task_id);
        self.ready_cache.clear();

        self.emit_event(TodoEvent::DependencyAdded {
            task_id: *task_id,
            depends_on: *depends_on,
        });

        Ok(())
    }

    /// Удалить зависимость между задачами
    pub async fn remove_dependency(&self, task_id: &Uuid, depends_on: &Uuid) -> Result<()> {
        // Удаляем из базы данных
        self.store.remove_dependency(task_id, depends_on).await?;

        // Обновляем граф
        if let Some(mut task) = self.get_cached(task_id).await? {
            task.depends_on.retain(|id| id != depends_on);
            self.graph.upsert_task(&task)?;

            // Проверяем нужно ли изменить состояние на Ready
            if task.state == TaskState::Blocked && self.graph.is_ready(task_id)? {
                self.update_state(task_id, TaskState::Ready).await?;
            }
        }

        // Инвалидируем кэши
        self.cache.lock().pop(task_id);
        self.ready_cache.clear();

        self.emit_event(TodoEvent::DependencyRemoved {
            task_id: *task_id,
            depends_on: *depends_on,
        });

        Ok(())
    }

    /// Поиск задач с полнотекстовым поиском
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<TodoItem>> {
        let results = self.store.search(query, limit).await?;

        // Добавляем в кэш
        {
            let mut cache = self.cache.lock();
            for task in &results {
                cache.put(task.id, task.clone());
            }
        }

        Ok(results)
    }

    /// Получить статистику
    pub async fn get_stats(&self) -> Result<(TaskStats, GraphStats)> {
        let task_stats = self.store.get_stats().await?;
        let graph_stats = self.graph.stats();
        Ok((task_stats, graph_stats))
    }

    /// Подписаться на события
    pub fn subscribe(&self) -> TodoEventStream {
        TodoEventStream {
            rx: self.events_rx.clone(),
        }
    }

    /// Отправить событие
    fn emit_event(&self, event: TodoEvent) {
        // Игнорируем ошибку если нет подписчиков
        let _ = self.events_tx.send(event);
    }

    /// Оптимизировать кэши и индексы
    pub async fn optimize(&self) -> Result<()> {
        // Очищаем устаревшие записи кэша
        self.ready_cache.clear();

        // Перестраиваем граф для оптимальной производительности
        let all_tasks = self.store.search("", usize::MAX).await?;
        self.graph.load_from_tasks(all_tasks)?;

        info!("Optimized caches and indexes");
        Ok(())
    }

    /// Обновить/добавить ключи в metadata задачи
    pub async fn upsert_metadata(&self, id: &Uuid, meta: HashMap<String, serde_json::Value>) -> Result<()> {
        self.store.update_metadata(id, meta).await
    }

    /// Добавить элемент в массив metadata задачи по ключу
    pub async fn push_metadata_item(&self, id: &Uuid, key: &str, element: serde_json::Value) -> Result<()> {
        self.store.append_metadata_array(id, key, element).await
    }
}

/// Поток событий для подписчиков
pub struct TodoEventStream {
    rx: Arc<Mutex<mpsc::UnboundedReceiver<TodoEvent>>>,
}

impl TodoEventStream {
    /// Получить следующее событие
    #[allow(clippy::await_holding_lock)]
    pub async fn next(&self) -> Option<TodoEvent> {
        self.rx.lock().recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_service() -> Result<TodoServiceV2> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        TodoServiceV2::new(db_path, 4, 100).await
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let service = Arc::new(create_test_service().await.unwrap());

        // Создаем задачи параллельно
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let svc = service.clone();
                tokio::spawn(async move {
                    svc.create_task(
                        format!("Task {}", i),
                        "Description".to_string(),
                        Priority::Medium,
                        vec![],
                    )
                    .await
                })
            })
            .collect();

        for h in handles {
            h.await.unwrap().unwrap();
        }

        let (stats, _) = service.get_stats().await.unwrap();
        assert_eq!(stats.total, 10);
    }

    #[tokio::test]
    async fn test_dependency_cascade() {
        let service = create_test_service().await.unwrap();

        // Создаем цепочку зависимых задач
        let task1 = service
            .create_task(
                "Task 1".to_string(),
                "First".to_string(),
                Priority::High,
                vec![],
            )
            .await
            .unwrap();

        let task2 = service
            .create_task(
                "Task 2".to_string(),
                "Second".to_string(),
                Priority::High,
                vec![],
            )
            .await
            .unwrap();

        service.add_dependency(&task2.id, &task1.id).await.unwrap();

        // task2 должна быть заблокирована
        let task2_updated = service.get_cached(&task2.id).await.unwrap().unwrap();
        assert_eq!(task2_updated.state, TaskState::Blocked);

        // Завершаем task1
        service
            .update_state(&task1.id, TaskState::Done)
            .await
            .unwrap();

        // task2 должна стать готовой
        let task2_ready = service.get_cached(&task2.id).await.unwrap().unwrap();
        assert_eq!(task2_ready.state, TaskState::Ready);
    }
}
