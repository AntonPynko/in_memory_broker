Topic – структура, содержащая канал Sender для рассылки сообщений подписчикам;
Broker – структура, основной управляющий элемент, который управляет топиками и подписчиками. Использует Arc и Mutex для потокобезопасного доступа к данным;
create_topic – метод, который создаёт новый топик и добавляет его в HashMap;
subscribe – метод, который добавляет подписчика на определённый топик и возвращает Receiver для получения сообщений;
is_subscribed - метод, проверяющий наличие подписки;
unsubscribe – метод, который удаляет подписчика из списка клиентов для указанного топика;
publish – метод, который публикует сообщение в топик и отправляет его всем подписчикам.
--------------------

Проверка работоспособности
1) Запустить сargo run
2) Создание топика -
curl -X POST http://localhost:3030/topic/my_topic

3) Подписка на топик через SSE
curl -N http://localhost:3030/topic/my_topic/subscribe/1

4) Публикация сообщения в топик
curl -X POST -d 'Hello, world!' http://localhost:3030/topic/my_topic/message

5) Отписаться с топика
curl -X POST http://localhost:3030/topic/my_topic/unsubscribe/1
