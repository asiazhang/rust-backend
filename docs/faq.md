# FAQ

## 1. 为什么使用`sqlx`作为数据库访问工具，而不使用其他的orm框架？

> **sqlx is not a orm.**

当前rust中常见的orm有：

- [`diesel`](https://diesel.rs/)
- [`seaorm`](https://www.sea-ql.org/SeaORM/)

不使用的原因是：

1. 个人不太喜欢使用ORM，喜欢更简单直接的方式。这样更透明，有什么业务逻辑一目了然。
2. 如果涉及比较复杂的查询，ORM也无法解决，仍然会退化到编写原始sql模式。
3. sqlx中涉及到`in`的问题，当前使用的是`postgres`，基本都有[解决方案](https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-do-a-select--where-foo-in--query)。

> diesel官方给出了[一篇文章](https://diesel.rs/compare_diesel.html)，指明diesel相对于其他sql工具的优缺点。
> 不过从我自己的观点来看，除了in的问题(postgres中有对应解决方案)，其余无伤大雅。


## 2. 为什么使用Postgresql作为数据库，而不是Mysql?

## 3. 为什么使用Redis作为消息队列，而不是Kafka/RabbitMQ?

Simple. 

对于中小型应用来说，大多数有以下特点：

- 访问量不高，不需要动态伸缩
- 对高可用性需求不强烈

在这种场景下，redis已经可以满足简单的消息队列用途，同时还能作为缓存系统使用。