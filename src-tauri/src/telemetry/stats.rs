//! 统计聚合器
//!
//! 提供请求统计的聚合、分组和查询功能

use crate::telemetry::types::{
    ModelStats, ProviderStats, RequestLog, RequestStatus, StatsSummary, TimeRange,
};
use crate::ProviderType;
use chrono::{Duration, Utc};
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};

/// 统计聚合器
///
/// 管理请求日志的统计聚合，支持按时间范围、Provider 和模型分组统计
pub struct StatsAggregator {
    /// 日志队列
    logs: RwLock<VecDeque<RequestLog>>,
    /// 日志保留时长
    retention: Duration,
    /// 最大日志条数
    max_logs: usize,
}

impl StatsAggregator {
    /// 创建新的统计聚合器
    ///
    /// # Arguments
    /// * `retention` - 日志保留时长
    /// * `max_logs` - 最大日志条数
    pub fn new(retention: Duration, max_logs: usize) -> Self {
        Self {
            logs: RwLock::new(VecDeque::with_capacity(max_logs)),
            retention,
            max_logs,
        }
    }

    /// 使用默认配置创建统计聚合器（保留 7 天，最多 10000 条）
    pub fn with_defaults() -> Self {
        Self::new(Duration::days(7), 10000)
    }

    /// 记录请求日志
    ///
    /// 将日志添加到聚合器中，并自动清理过期日志
    pub fn record(&self, log: RequestLog) {
        let mut logs = self.logs.write();
        logs.push_back(log);

        // 清理超出数量限制的日志
        while logs.len() > self.max_logs {
            logs.pop_front();
        }

        // 清理过期日志
        let cutoff = Utc::now() - self.retention;
        while let Some(front) = logs.front() {
            if front.timestamp < cutoff {
                logs.pop_front();
            } else {
                break;
            }
        }
    }

    /// 获取统计摘要
    ///
    /// # Arguments
    /// * `range` - 可选的时间范围，如果为 None 则统计所有日志
    pub fn summary(&self, range: Option<TimeRange>) -> StatsSummary {
        let logs = self.get_logs_in_range(range);
        StatsSummary::from_logs(&logs)
    }

    /// 获取指定时间范围内的日志
    fn get_logs_in_range(&self, range: Option<TimeRange>) -> Vec<RequestLog> {
        let logs = self.logs.read();
        match range {
            Some(r) => logs
                .iter()
                .filter(|l| r.contains(&l.timestamp))
                .cloned()
                .collect(),
            None => logs.iter().cloned().collect(),
        }
    }

    /// 获取所有日志
    pub fn get_all(&self) -> Vec<RequestLog> {
        self.logs.read().iter().cloned().collect()
    }

    /// 获取日志数量
    pub fn len(&self) -> usize {
        self.logs.read().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.logs.read().is_empty()
    }

    /// 清空所有日志
    pub fn clear(&self) {
        self.logs.write().clear();
    }

    /// 清理过期日志
    ///
    /// 返回清理的日志数量
    pub fn cleanup_expired(&self) -> usize {
        let mut logs = self.logs.write();
        let cutoff = Utc::now() - self.retention;
        let initial_len = logs.len();

        while let Some(front) = logs.front() {
            if front.timestamp < cutoff {
                logs.pop_front();
            } else {
                break;
            }
        }

        initial_len - logs.len()
    }
}

impl Default for StatsAggregator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ========== 分组统计方法 ==========

impl StatsAggregator {
    /// 按 Provider 分组统计
    ///
    /// # Arguments
    /// * `range` - 可选的时间范围
    ///
    /// # Returns
    /// 按 Provider 分组的统计数据
    pub fn by_provider(&self, range: Option<TimeRange>) -> HashMap<ProviderType, ProviderStats> {
        let logs = self.get_logs_in_range(range);

        // 按 Provider 分组
        let mut grouped: HashMap<ProviderType, Vec<RequestLog>> = HashMap::new();
        for log in logs {
            grouped.entry(log.provider).or_default().push(log);
        }

        // 计算每个 Provider 的统计
        grouped
            .into_iter()
            .map(|(provider, logs)| (provider, ProviderStats::from_logs(provider, &logs)))
            .collect()
    }

    /// 按模型分组统计
    ///
    /// # Arguments
    /// * `range` - 可选的时间范围
    ///
    /// # Returns
    /// 按模型名称分组的统计数据
    pub fn by_model(&self, range: Option<TimeRange>) -> HashMap<String, ModelStats> {
        let logs = self.get_logs_in_range(range);

        // 按模型分组
        let mut grouped: HashMap<String, Vec<RequestLog>> = HashMap::new();
        for log in logs {
            grouped.entry(log.model.clone()).or_default().push(log);
        }

        // 计算每个模型的统计
        grouped
            .into_iter()
            .map(|(model, logs)| {
                let stats = ModelStats::from_logs(model.clone(), &logs);
                (model, stats)
            })
            .collect()
    }

    /// 按 Provider 和模型分组统计
    ///
    /// # Arguments
    /// * `range` - 可选的时间范围
    ///
    /// # Returns
    /// 按 (Provider, Model) 分组的统计数据
    pub fn by_provider_and_model(
        &self,
        range: Option<TimeRange>,
    ) -> HashMap<(ProviderType, String), StatsSummary> {
        let logs = self.get_logs_in_range(range);

        // 按 (Provider, Model) 分组
        let mut grouped: HashMap<(ProviderType, String), Vec<RequestLog>> = HashMap::new();
        for log in logs {
            grouped
                .entry((log.provider, log.model.clone()))
                .or_default()
                .push(log);
        }

        // 计算每个组的统计
        grouped
            .into_iter()
            .map(|(key, logs)| (key, StatsSummary::from_logs(&logs)))
            .collect()
    }

    /// 按状态分组统计
    ///
    /// # Arguments
    /// * `range` - 可选的时间范围
    ///
    /// # Returns
    /// 按请求状态分组的统计数据
    pub fn by_status(&self, range: Option<TimeRange>) -> HashMap<RequestStatus, u64> {
        let logs = self.get_logs_in_range(range);

        let mut grouped: HashMap<RequestStatus, u64> = HashMap::new();
        for log in logs {
            *grouped.entry(log.status).or_default() += 1;
        }

        grouped
    }

    /// 获取指定 Provider 的统计
    ///
    /// # Arguments
    /// * `provider` - Provider 类型
    /// * `range` - 可选的时间范围
    pub fn provider_stats(
        &self,
        provider: ProviderType,
        range: Option<TimeRange>,
    ) -> ProviderStats {
        let logs = self.get_logs_in_range(range);
        let filtered: Vec<RequestLog> = logs
            .into_iter()
            .filter(|l| l.provider == provider)
            .collect();
        ProviderStats::from_logs(provider, &filtered)
    }

    /// 获取指定模型的统计
    ///
    /// # Arguments
    /// * `model` - 模型名称
    /// * `range` - 可选的时间范围
    pub fn model_stats(&self, model: &str, range: Option<TimeRange>) -> ModelStats {
        let logs = self.get_logs_in_range(range);
        let filtered: Vec<RequestLog> = logs.into_iter().filter(|l| l.model == model).collect();
        ModelStats::from_logs(model.to_string(), &filtered)
    }
}
