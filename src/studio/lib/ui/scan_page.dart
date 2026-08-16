/// 扫描页——DevOps 工作台首页（子模块扫描 + 状态列表）。
library;

import 'package:flutter/material.dart';

import '../api/cli_client.dart';
import '../app_state.dart';

class ScanPage extends StatelessWidget {
  final AppState state;

  const ScanPage({super.key, required this.state});

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: state,
      builder: (context, _) {
        return Scaffold(
          backgroundColor: const Color(0xFFF1F5F9),
          body: SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(28),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    '量潮 DevOps 云',
                    style: TextStyle(
                      fontSize: 24,
                      fontWeight: FontWeight.w700,
                      color: Color(0xFF1E293B),
                    ),
                  ),
                  const SizedBox(height: 4),
                  const Text(
                    '子模块扫描 · 构建 · 发布',
                    style: TextStyle(fontSize: 13, color: Color(0xFF94A3B8)),
                  ),
                  const SizedBox(height: 20),
                  _buildSummaryCard(),
                  if (state.lastScanError != null) ...[
                    const SizedBox(height: 12),
                    _buildErrorBanner(),
                  ],
                  const SizedBox(height: 16),
                  Expanded(child: _buildComponentList()),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _buildSummaryCard() {
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  '子模块状态',
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w700,
                    color: Color(0xFF1E293B),
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  state.lastScanSummary ?? '未扫描',
                  style: const TextStyle(fontSize: 12, color: Color(0xFF64748B)),
                ),
              ],
            ),
          ),
          FilledButton(
            onPressed: state.scanning ? null : state.scan,
            child: state.scanning
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Text('扫描'),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorBanner() {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFFFEF2F2),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: const Color(0xFFFECACA)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Icon(Icons.warning_amber_rounded,
              size: 18, color: Color(0xFFDC2626)),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              state.lastScanError!,
              style: const TextStyle(fontSize: 12, color: Color(0xFFB91C1C)),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildComponentList() {
    final components = state.components;
    if (components.isEmpty) {
      return Container(
        width: double.infinity,
        padding: const EdgeInsets.all(24),
        decoration: BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.circular(12),
        ),
        child: const Text(
          '暂无子模块数据——点击「扫描」调用 qtcloud-devops CLI 获取状态',
          style: TextStyle(fontSize: 12, color: Color(0xFF94A3B8)),
        ),
      );
    }
    return ListView.separated(
      itemCount: components.length,
      separatorBuilder: (_, _) => const SizedBox(height: 8),
      itemBuilder: (context, index) {
        final component = components[index];
        return Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          decoration: BoxDecoration(
            color: Colors.white,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Row(
            children: [
              Container(
                width: 10,
                height: 10,
                decoration: BoxDecoration(
                  color: _statusColor(component.status),
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  component.name,
                  style: const TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: Color(0xFF1E293B),
                  ),
                ),
              ),
              if (component.ahead > 0 || component.behind > 0)
                Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Text(
                    _syncDetail(component),
                    style: const TextStyle(fontSize: 12, color: Color(0xFF64748B)),
                  ),
                ),
              Text(
                component.statusLabel,
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                  color: _statusColor(component.status),
                ),
              ),
            ],
          ),
        );
      },
    );
  }

  Color _statusColor(ComponentSyncStatus status) => switch (status) {
        ComponentSyncStatus.synced => const Color(0xFF16A34A),
        ComponentSyncStatus.pendingPush => const Color(0xFF2563EB),
        ComponentSyncStatus.pendingPull => const Color(0xFFD97706),
        ComponentSyncStatus.conflict => const Color(0xFFDC2626),
      };

  String _syncDetail(ScanComponent component) {
    if (component.ahead > 0 && component.behind > 0) {
      return '+${component.ahead}/-${component.behind}';
    }
    if (component.ahead > 0) return '领先 ${component.ahead} 提交';
    return '落后 ${component.behind} 提交';
  }
}
