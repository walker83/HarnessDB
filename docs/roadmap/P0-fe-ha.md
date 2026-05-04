# P0: Frontend 高可用

**优先级**: P0 (核心缺失)
**模块**: fe-common
**状态**: ❌ 未开始

## 背景

RorisDB 当前 FE 为单节点，无高可用保障。FE 宕机后整个集群不可用。需要实现基于 Raft 的 FE 多副本共识机制。

## 任务清单

### 1. Raft 基础库集成
- [ ] 选择或实现 Raft 库（如 raft-rs, openraft）
- [ ] 实现 LogEntry、LogStore、StateMachine 基础接口
- [ ] 实现 Leader 选举
- [ ] 实现 Log 复制
- [ ] 实现 Snapshot 机制

### 2. 元数据共识
- [ ] 将 EditLog 操作包装为 Raft LogEntry
- [ ] Master FE 接收写请求 → 复制到 Follower → 写入本地
- [ ] Follower FE 从 Master 同步元数据
- [ ] 元数据包括：Database、Table、Partition、Replica 信息

### 3. Master 选举
- [ ] FE 启动时发起选举
- [ ] Leader 切换时保证元数据一致性
- [ ] Follower 转为 Master 的无缝切换
- [ ] 支持配置 Observer 节点（不参与选举，只同步数据）

### 4. Quorum 协议
- [ ] 写操作需获得多数派确认
- [ ] 读操作可从 Master 本地读取
- [ ] 网络分区时的正确行为（少数派不可写）

### 5. 集群管理
- [ ] `ALTER SYSTEM ADD/DROP FOLLOWER/OBSERVER` 命令
- [ ] FE 节点状态管理（Online/Offline/Recovering）
- [ ] Master 信息展示（`SHOW FRONTENDS`）

### 6. 故障恢复
- [ ] FE 重启后从 Raft Log 恢复状态
- [ ] 新 FE 加入集群后全量同步元数据
- [ ] BDBJE 元数据兼容（如果从 Doris 迁移）

## 涉及文件

- `crates/fe-common/src/raft.rs` - 新建，Raft 共识层
- `crates/fe-common/src/edit_log.rs` - 修改，EditLog 通过 Raft 复制
- `crates/fe-common/src/meta_service.rs` - 修改，元数据服务适配 HA
- `crates/fe-catalog/src/` - 修改，Catalog 操作适配 HA
