# FAQ

## 1. 为什么使用`sqlx`作为数据库访问工具，而不使用其他的orm框架？

> **sqlx is not a orm.**

当前rust中常见的orm有：

- [`diesel`](https://diesel.rs/)
- [`seaorm`](https://www.sea-ql.org/SeaORM/)

不使用的原因是：

1. 个人不太喜欢使用ORM，喜欢更简单直接的方式。这样更透明，啥业务逻辑一目了然。
2. 如果涉及比较复杂的查询，ORM也无法解决，仍然会退化到编写原始sql模式。
3. sqlx中涉及到`in`的问题，当前使用的是`postgres`，基本都有[解决方案](https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-do-a-select--where-foo-in--query)。