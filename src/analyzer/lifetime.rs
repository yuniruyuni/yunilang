//! ライフタイム解析関連の構造と処理

use crate::ast::{Span, Type};
use std::collections::{HashMap, HashSet};

use super::symbol::{AnalysisError, AnalysisResult, BorrowInfo, BorrowKind, LifetimeId};

/// ライフタイム情報
#[derive(Debug)]
#[allow(dead_code)]
pub struct Lifetime {
    /// ライフタイムID
    pub id: LifetimeId,
    /// このライフタイムが依存するライフタイム（このライフタイムより長く生きる必要がある）
    pub outlives: HashSet<LifetimeId>,
    /// このライフタイムのスコープ開始位置
    pub start_scope: ScopeId,
    /// このライフタイムのスコープ終了位置
    pub end_scope: Option<ScopeId>,
    /// ライフタイムが定義された場所
    pub span: Span,
}

/// スコープID（ネストしたスコープを管理）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// 変数の使用情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct VariableUsage {
    /// 使用の種類
    pub usage_kind: UsageKind,
    /// 使用された場所
    pub span: Span,
    /// 使用されたスコープ
    pub scope: ScopeId,
}

/// 変数の使用種類
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum UsageKind {
    /// 読み取り
    Read,
    /// 書き込み
    Write,
    /// 借用
    Borrow(BorrowKind),
    /// 移動
    Move,
}

/// ライフタイム制約の種類
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LivesConstraint {
    /// 'a: 'b (aはbより長く生きる必要がある)
    Outlives {
        longer: LifetimeId,
        shorter: LifetimeId,
        span: Span,
    },
    /// 型Tが特定のライフタイムより長く生きる必要がある
    TypeOutlives {
        ty: Type,
        lifetime: LifetimeId,
        span: Span,
    },
}

/// ライフタイム分析のコンテキスト
#[derive(Debug)]
#[allow(dead_code)]
pub struct LifetimeContext {
    /// 全てのライフタイム
    pub lifetimes: HashMap<LifetimeId, Lifetime>,
    /// ライフタイム制約
    pub constraints: Vec<LivesConstraint>,
    /// 現在のスコープID
    pub current_scope: ScopeId,
    /// スコープの階層構造（parent scope mapping）
    pub scope_hierarchy: HashMap<ScopeId, Option<ScopeId>>,
    /// 次のスコープID
    pub next_scope_id: usize,
    /// 次の無名ライフタイムID
    pub next_anonymous_id: usize,
    /// 変数の借用情報
    pub variable_borrows: HashMap<String, Vec<BorrowInfo>>,
    /// 変数の使用履歴
    pub variable_usage: HashMap<String, Vec<VariableUsage>>,
}

#[allow(dead_code)]
impl LifetimeContext {
    pub fn new() -> Self {
        let mut ctx = Self {
            lifetimes: HashMap::new(),
            constraints: Vec::new(),
            current_scope: ScopeId(0),
            scope_hierarchy: HashMap::new(),
            next_scope_id: 1,
            next_anonymous_id: 0,
            variable_borrows: HashMap::new(),
            variable_usage: HashMap::new(),
        };
        
        // 静的ライフタイムを登録
        ctx.register_static_lifetime();
        
        ctx
    }
    
    /// 静的ライフタイムを登録
    fn register_static_lifetime(&mut self) {
        let static_lifetime = Lifetime {
            id: LifetimeId::Static,
            outlives: HashSet::new(),
            start_scope: ScopeId(0),
            end_scope: None, // 静的ライフタイムは終了しない
            span: Span::dummy(),
        };
        self.lifetimes.insert(LifetimeId::Static, static_lifetime);
    }
    
    /// 新しいスコープを開始
    pub fn enter_scope(&mut self) -> ScopeId {
        let new_scope = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;
        
        // 親スコープを記録
        self.scope_hierarchy.insert(new_scope, Some(self.current_scope));
        self.current_scope = new_scope;
        
        new_scope
    }
    
    /// スコープを終了
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.scope_hierarchy.get(&self.current_scope).cloned().flatten() {
            self.current_scope = parent;
        }
    }
    
    /// 新しい無名ライフタイムを生成
    pub fn create_anonymous_lifetime(&mut self, span: Span) -> LifetimeId {
        let id = LifetimeId::Named(self.next_anonymous_id);
        self.next_anonymous_id += 1;
        
        let lifetime = Lifetime {
            id,
            outlives: HashSet::new(),
            start_scope: self.current_scope,
            end_scope: None,
            span,
        };
        
        self.lifetimes.insert(id, lifetime);
        id
    }
    
    /// 名前付きライフタイムを登録
    pub fn register_named_lifetime(&mut self, _name: String, span: Span) -> AnalysisResult<LifetimeId> {
        // 簡単のため、名前付きライフタイムも番号で管理
        let id = LifetimeId::Named(self.next_anonymous_id);
        self.next_anonymous_id += 1;
        
        let lifetime = Lifetime {
            id,
            outlives: HashSet::new(),
            start_scope: self.current_scope,
            end_scope: None,
            span,
        };
        
        self.lifetimes.insert(id, lifetime);
        Ok(id)
    }
    
    /// ライフタイム制約を追加
    pub fn add_constraint(&mut self, constraint: LivesConstraint) {
        self.constraints.push(constraint);
    }
    
    /// ライフタイムの依存関係を追加（'a: 'b means 'a outlives 'b）
    pub fn add_outlives_constraint(&mut self, longer: LifetimeId, shorter: LifetimeId) {
        if let Some(lifetime) = self.lifetimes.get_mut(&shorter) {
            lifetime.outlives.insert(longer);
        }
    }
    
    /// 変数の借用を記録
    pub fn record_borrow(&mut self, var_name: String, kind: BorrowKind, lifetime: LifetimeId, _span: Span) {
        let borrow_info = BorrowInfo {
            kind,
            lifetime,
        };
        
        self.variable_borrows.entry(var_name).or_default().push(borrow_info);
    }
    
    /// 変数の使用を記録
    pub fn record_usage(&mut self, var_name: String, usage_kind: UsageKind, span: Span) {
        let usage = VariableUsage {
            usage_kind,
            span,
            scope: self.current_scope,
        };
        
        self.variable_usage.entry(var_name).or_default().push(usage);
    }
    
    /// 借用チェックを実行
    pub fn check_borrows(&self) -> AnalysisResult<()> {
        // 各変数について、借用ルールを検証
        for (var_name, borrows) in &self.variable_borrows {
            // 可変借用は同時に1つまで
            let mutable_borrows: Vec<_> = borrows.iter()
                .filter(|b| b.kind == BorrowKind::Mutable)
                .collect();
            
            if mutable_borrows.len() > 1 {
                return Err(AnalysisError::MultipleMutableBorrows {
                    name: var_name.clone(),
                    span: Span::dummy(),
                });
            }
            
            // 可変借用と不変借用は同時に存在できない
            let has_mutable = borrows.iter().any(|b| b.kind == BorrowKind::Mutable);
            let has_immutable = borrows.iter().any(|b| b.kind == BorrowKind::Shared);
            
            if has_mutable && has_immutable {
                return Err(AnalysisError::MutableBorrowConflict {
                    name: var_name.clone(),
                    span: Span::dummy(),
                });
            }
        }
        
        Ok(())
    }
    
    /// ライフタイム制約を検証
    pub fn verify_constraints(&self) -> AnalysisResult<()> {
        for constraint in &self.constraints {
            match constraint {
                LivesConstraint::Outlives { longer, shorter, span } => {
                    // 制約の検証ロジック
                    // ここでは簡略化して、制約が存在することだけを確認
                    if !self.lifetimes.contains_key(longer) || !self.lifetimes.contains_key(shorter) {
                        return Err(AnalysisError::LifetimeError {
                            message: "未定義のライフタイムが制約に含まれています".to_string(),
                            span: *span,
                        });
                    }
                }
                LivesConstraint::TypeOutlives { ty, lifetime, span } => {
                    // 型のライフタイム制約を検証
                    if !self.lifetimes.contains_key(lifetime) {
                        return Err(AnalysisError::LifetimeError {
                            message: format!("型 {:?} に対する未定義のライフタイム制約", ty),
                            span: *span,
                        });
                    }
                }
            }
        }
        
        Ok(())
    }
}