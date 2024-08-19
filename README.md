# FileOrganizer: Структура и Возможности

## Потенциальные Возможности

### **Слушатели Событий**:

- Возможность повесить слушатели на определенные папки, например, папку `Downloads`.
- Слушатели будут обрабатывать файлы на основе условий.

### **Условия Обработки**:

- Тип файла (изображения, музыка и т.д.).
- Соответствие имени файла регулярному выражению.
- Размер файла.
- Другие метаданные.

### **Действия при Событии**:

- Переместить файл в другую папку.
- Создать симлинк на файл (например, изображения больше 2K могут иметь симлинк в папку `images/2K+`).
- Удалить файл или переместить в корзину (например, при достижении определенного размера).

### **Потенциальные Дополнительные Функции**:

- Архивирование (например, лог-файлов).
- Выполнение кастомных команд (например, `rsync` для перемещения файлов).
- Расширенная поддержка типов файлов для специфичных условий (например, длина аудио треков, обработка видео файлов и т.д.).

### **Комбинирование Действий**:

- Слушатели событий могут работать на разных уровнях: например, папка Загрузки -> Изображения -> Обработка изображений на основе условий.
- Создание сценариев для поддержания файловой системы в структурированном состоянии.

## Зависимости

- **Библиотеки для Определения Типов и Метаданных Файлов**:

  - Для работы с изображениями, музыкой и другими типами файлов.

- **Асинхронное Программирование**:

  - `tokio` и сопутствующие библиотеки для асинхронного рантайма.

- **Акторы**:

  - `elfo` или другие библиотеки акторов для реализации акторной модели.

- **База Данных**:
  - `sqlx` в связке с `sqlite` для хранения информации о символических ссылках и метаданных.

## Структура Проекта (Версия на Акторах)

### Акторы

Возомжно есть смысл пересмотреть структуру акторов, к примеру акторов по работе с фс обьединить, или на оборот разбить еще сильнее, добавить акторов удалиния и т.д.

- **Слушатели Событий**:

  Каждый актор слушает события в определенной папке.
  Можно добавлять правила для работы с регулярными выражениями.
  Возможность добавления логических блоков для обработки изображений, перемещения файлов по размерам, дате и т.д.

- **Акторы для Перемещения Файлов**:

  Ответственные за перемещение файлов в соответствии с условиями и правилами.

- **Акторы для Линковки Файлов**:

  Ответственные за создание символических ссылок на файлы.

- **Системные Акторы Конфигураторы**:

  Автоматическое обновление конфигурации и пересоздание слушателей на лету.

- **Актор для Обработки конкретного типа**:

  Проверка файла на соответствие условиям, 2й ряд условий уже более специфичных для конерктного типа.
  Генерация задач исходя из конфигурации и условий.

- **Актор Базы Данных**:

  Хранение информации о символических ссылках.
  Удаление/обновление симлинков при удалении/перемещении/переименовании файлов.

- **Актор Сервера** (опционально):

  Возможный фронтенд для управления и настройки.

- **Актор Архиватор** (опционально):

  Архивирование логов и других данных.

- **Актор Нотификатор** (опционально):
  Уведомление пользователей о событиях файловой системы.

## Реализация

1. **База данных**:

   - Создание таблицы для хранения символических ссылок и их соответствий.
   - Реализация функций для добавления, удаления и получения симлинков.

2. **Обработка Файлов**:

   - Реализация функций для создания симлинков и их удаления при необходимости.
   - Обработка файлов на основе условий и регулярных выражений.

3. **Асинхронное Обработка**:

   - Использование `tokio` для асинхронного выполнения задач и обработки событий файловой системы.

4. **Акторная Модель**:

   - Реализация акторов для управления различными аспектами обработки файлов и симлинков.

5. **Конфигурация**:
   - Создание конфигурационных файлов для определения правил обработки и управления слушателями.