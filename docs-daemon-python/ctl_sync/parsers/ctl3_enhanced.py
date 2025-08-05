"""
CTL v3.0 Enhanced Parser - Улучшенная версия с расширенной семантикой

Поддерживает:
- Вложенные тензорные выражения
- Условные операторы (∵, ∴) 
- Параллельные и последовательные композиции
- Автоматическую миграцию v2→v3
- Расширенную валидацию
"""

# @component: {"k":"C","id":"ctl3_enhanced_parser","t":"Enhanced CTL v3.0 tensor parser","m":{"cur":75,"tgt":100,"u":"%"},"f":["python","ctl3","parser","tensor","enhanced"]}

import re
import json
from typing import Any, Dict, List, Optional, Tuple, Union
from dataclasses import dataclass, field
from enum import Enum

from .base_parser import BaseParser
from ..schema import CtlSchema


class TensorOperator(Enum):
    """Расширенный набор тензорных операторов"""
    # Композиция и операции
    COMPOSE = ('⊗', 'compose', 'Тензорное произведение')
    PARALLEL = ('⊕', 'parallel', 'Параллельная композиция')
    ELEMENTWISE = ('⊙', 'elementwise', 'Поэлементное произведение')
    CONVOLUTION = ('⊡', 'convolve', 'Свертка')
    
    # Оптимизация и направления
    GRADIENT = ('∇', 'grad', 'Градиент/Оптимизация')
    PARTIAL = ('∂', 'partial', 'Частная производная')
    
    # Логические связи
    THEREFORE = ('∴', 'therefore', 'Следовательно')
    BECAUSE = ('∵', 'because', 'Поскольку')
    EQUIVALENT = ('≡', 'equiv', 'Эквивалентность')
    IMPLIES = ('⟹', 'implies', 'Импликация')
    BIDIRECTIONAL = ('⟷', 'bidir', 'Двунаправленная связь')
    
    # Новые операторы
    SEQUENTIAL = ('◦', 'seq', 'Последовательная композиция')
    CROSS_PRODUCT = ('⊠', 'cross', 'Кросс-произведение')
    JOIN = ('⋈', 'join', 'Соединение')
    INTERSECTION = ('∩', 'intersect', 'Пересечение')
    UNION = ('∪', 'union', 'Объединение')
    APPROXIMATION = ('≈', 'approx', 'Приближенная эквивалентность')
    
    def __init__(self, unicode: str, ascii: str, description: str):
        self.unicode = unicode
        self.ascii = ascii
        self.description = description


@dataclass
class TensorExpression:
    """Структура для представления тензорного выражения"""
    operator: Optional[TensorOperator] = None
    operands: List[Union[str, 'TensorExpression']] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        """Преобразование в словарь для JSON"""
        result = {}
        if self.operator:
            result['op'] = self.operator.ascii
        if self.operands:
            result['operands'] = [
                op.to_dict() if isinstance(op, TensorExpression) else op
                for op in self.operands
            ]
        if self.metadata:
            result.update(self.metadata)
        return result


class Ctl3EnhancedParser(BaseParser):
    """Улучшенный парсер CTL v3.0 с расширенной семантикой"""
    
    def __init__(self):
        super().__init__()
        self.schema = CtlSchema()
        self.setup_patterns()  # Call base class requirement
        self.setup_enhanced_patterns()
    
    def setup_patterns(self) -> None:
        """Setup base patterns (required by BaseParser)"""
        # This is handled in setup_enhanced_patterns
        pass
        
    def setup_enhanced_patterns(self) -> None:
        """Настройка расширенных паттернов парсинга"""
        # Основной паттерн CTL v3.0 (поддержка Unicode и ASCII)
        # Ⱦ может быть представлен как D, а специальный символ ловим отдельно
        self.ctl3_pattern = re.compile(
            r'//\s*@(?:ctl3|tensor|component3):\s*([Dⱦ]\[.*?\]\s*:=\s*\{.*?\})',
            re.IGNORECASE | re.DOTALL
        )
        
        # Паттерн для компонента с метаданными
        self.component_pattern = re.compile(
            r'([Dⱦ])\[([^:]+):([^\]]+)\](?:\s*<([^>]+)>)?\s*:=\s*\{(.*?)\}',
            re.DOTALL
        )
        
        # Паттерны для различных операций
        self.maturity_pattern = re.compile(r'(?:∇|grad)\[(\d+)(?:→|->)(\d+)(?:,\s*([^]]+))?\]')
        self.dependency_pattern = re.compile(r'(?:⊗|compose)\[([^\]]+)\]')
        self.parallel_pattern = re.compile(r'(?:⊕|parallel)\[([^\]]+)\]')
        self.condition_pattern = re.compile(r'(?:∵|because)\s*([^∴]+)(?:∴|therefore)\s*(.+)')
        
        # Паттерн для вложенных выражений
        self.nested_pattern = re.compile(r'\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}')
        
        # Создаем маппинг операторов
        self.operator_map = {
            op.unicode: op for op in TensorOperator
        }
        self.operator_map.update({
            op.ascii: op for op in TensorOperator
        })
    
    def parse_line(self, line: str, line_no: int, file_path: str) -> Optional[Dict[str, Any]]:
        """Парсинг строки с CTL v3.0 аннотацией"""
        match = self.ctl3_pattern.search(line)
        if not match:
            # Пробуем найти CTL v2.0 для автоматической миграции
            return self.try_migrate_v2_to_v3(line, line_no, file_path)
        
        tensor_str = match.group(1)
        component = self.parse_enhanced_tensor(tensor_str, file_path, line_no)
        
        if component:
            self.add_file_location(component, file_path, line_no)
            self.validate_and_enhance(component)
        
        return component
    
    def parse_enhanced_tensor(self, tensor_str: str, file_path: str, line_no: int) -> Optional[Dict[str, Any]]:
        """Расширенный парсинг тензорного выражения"""
        match = self.component_pattern.search(tensor_str)
        if not match:
            return None
        
        kind_symbol = match.group(1)
        component_id = match.group(2).strip()
        component_type = match.group(3).strip()
        metadata = match.group(4) if match.group(4) else None
        operations = match.group(5).strip()
        
        # Базовый компонент
        component = {
            'k': self.infer_kind(component_type, kind_symbol),
            'id': component_id,
            't': component_type,
            'ctl3_tensor': tensor_str,
            'v': 3  # Версия CTL
        }
        
        # Парсим метаданные если есть
        if metadata:
            component['meta'] = self.parse_metadata(metadata)
        
        # Парсим тензорные операции
        tensor_expr = self.parse_tensor_expression(operations)
        if tensor_expr:
            component['tensor'] = tensor_expr.to_dict()
        
        # Извлекаем стандартные поля
        self._extract_enhanced_maturity(component, operations)
        self._extract_enhanced_dependencies(component, operations)
        self._extract_enhanced_flags(component, operations)
        self._extract_conditions(component, operations)
        
        return component
    
    def parse_tensor_expression(self, expr_str: str) -> Optional[TensorExpression]:
        """Парсинг вложенных тензорных выражений"""
        expr = TensorExpression()
        
        # Ищем операторы и их операнды
        for op_symbol, operator in self.operator_map.items():
            if op_symbol in expr_str:
                expr.operator = operator
                
                # Извлекаем операнды в квадратных скобках
                pattern = re.compile(f'{re.escape(op_symbol)}\\[([^\\]]+)\\]')
                match = pattern.search(expr_str)
                if match:
                    operands_str = match.group(1)
                    # Разбираем операнды (могут быть вложенными)
                    expr.operands = self.parse_operands(operands_str)
                break
        
        # Извлекаем дополнительные метаданные
        if '≈' in expr_str or 'approx' in expr_str:
            expr.metadata['approximation'] = True
        
        if '∴' in expr_str or 'therefore' in expr_str:
            expr.metadata['conclusion'] = True
            
        return expr if expr.operator or expr.operands else None
    
    def parse_operands(self, operands_str: str) -> List[Union[str, TensorExpression]]:
        """Парсинг операндов (могут быть вложенными выражениями)"""
        operands = []
        
        # Проверяем на вложенные выражения
        if '{' in operands_str:
            # Парсим вложенное выражение
            nested_match = self.nested_pattern.search(operands_str)
            if nested_match:
                nested_expr = self.parse_tensor_expression(nested_match.group(1))
                if nested_expr:
                    operands.append(nested_expr)
        else:
            # Простые операнды через запятую
            for op in operands_str.split(','):
                op = op.strip()
                if op:
                    operands.append(op)
        
        return operands
    
    def parse_metadata(self, metadata_str: str) -> Dict[str, Any]:
        """Парсинг метаданных компонента"""
        metadata = {}
        
        # Парсим пары ключ=значение
        pairs = metadata_str.split(',')
        for pair in pairs:
            if '=' in pair:
                key, value = pair.split('=', 1)
                key = key.strip()
                value = value.strip()
                
                # Пробуем преобразовать в правильный тип
                if value.lower() in ('true', 'false'):
                    metadata[key] = value.lower() == 'true'
                elif value.isdigit():
                    metadata[key] = int(value)
                elif value.replace('.', '').isdigit():
                    metadata[key] = float(value)
                else:
                    metadata[key] = value
        
        return metadata
    
    def _extract_enhanced_maturity(self, component: Dict[str, Any], operations: str) -> None:
        """Расширенное извлечение метрик готовности"""
        match = self.maturity_pattern.search(operations)
        if match:
            cur = int(match.group(1))
            tgt = int(match.group(2))
            confidence = match.group(3) if match.group(3) else None
            
            component['m'] = {
                'cur': cur,
                'tgt': tgt,
                'u': '%'
            }
            
            if confidence:
                # Может быть указана уверенность или приоритет
                if confidence.startswith('conf:'):
                    component['m']['confidence'] = float(confidence.replace('conf:', ''))
                elif confidence.startswith('P'):
                    component['m']['priority'] = confidence
    
    def _extract_enhanced_dependencies(self, component: Dict[str, Any], operations: str) -> None:
        """Расширенное извлечение зависимостей с типами"""
        # Композиционные зависимости
        compose_match = self.dependency_pattern.search(operations)
        if compose_match:
            deps_str = compose_match.group(1)
            dependencies = []
            
            for dep in deps_str.split(','):
                dep = dep.strip()
                # Проверяем на типизированные зависимости
                if ':' in dep:
                    dep_name, dep_type = dep.split(':', 1)
                    dependencies.append({
                        'name': dep_name.strip(),
                        'type': dep_type.strip()
                    })
                else:
                    dependencies.append(dep)
            
            component['d'] = dependencies
        
        # Параллельные зависимости
        parallel_match = self.parallel_pattern.search(operations)
        if parallel_match:
            parallel_str = parallel_match.group(1)
            component['parallel'] = [p.strip() for p in parallel_str.split(',')]
    
    def _extract_enhanced_flags(self, component: Dict[str, Any], operations: str) -> None:
        """Расширенное извлечение флагов с семантикой"""
        flags = set()
        
        # Проверяем все операторы
        for operator in TensorOperator:
            if operator.unicode in operations or operator.ascii in operations:
                flags.add(operator.ascii)
        
        # Дополнительные семантические флаги
        operations_lower = operations.lower()
        
        # Технологии
        if any(kw in operations_lower for kw in ['gpu', 'cuda', 'opencl', 'vulkan']):
            flags.add('gpu')
        if any(kw in operations_lower for kw in ['simd', 'avx', 'sse']):
            flags.add('simd')
        if any(kw in operations_lower for kw in ['ai', 'ml', 'neural', 'embedding']):
            flags.add('ai')
        
        # Архитектурные паттерны
        if 'async' in operations_lower or '⊕' in operations:
            flags.add('async')
        if 'stream' in operations_lower:
            flags.add('streaming')
        if 'batch' in operations_lower:
            flags.add('batch_processing')
        if 'cache' in operations_lower:
            flags.add('caching')
        if 'pool' in operations_lower:
            flags.add('pooling')
        
        # Качества
        if 'real_time' in operations_lower or 'realtime' in operations_lower:
            flags.add('real_time')
        if 'production' in operations_lower:
            flags.add('production')
        if 'experimental' in operations_lower:
            flags.add('experimental')
        
        if flags:
            component['f'] = sorted(list(flags))
    
    def _extract_conditions(self, component: Dict[str, Any], operations: str) -> None:
        """Извлечение условных выражений (∵...∴...)"""
        match = self.condition_pattern.search(operations)
        if match:
            cause = match.group(1).strip()
            effect = match.group(2).strip()
            
            component['conditions'] = {
                'cause': cause,
                'effect': effect
            }
    
    def try_migrate_v2_to_v3(self, line: str, line_no: int, file_path: str) -> Optional[Dict[str, Any]]:
        """Автоматическая миграция CTL v2.0 → v3.0"""
        # Паттерн для CTL v2.0
        v2_pattern = re.compile(r'//\s*@component:\s*(\{.*?\})', re.DOTALL)
        match = v2_pattern.search(line)
        
        if not match:
            return None
        
        try:
            v2_component = json.loads(match.group(1))
            
            # Создаем v3 компонент
            v3_tensor = self.convert_v2_to_v3_tensor(v2_component)
            
            # Добавляем маркер миграции
            v2_component['migrated_from_v2'] = True
            v2_component['ctl3_tensor'] = v3_tensor
            v2_component['v'] = 3
            
            return v2_component
            
        except json.JSONDecodeError:
            return None
    
    def convert_v2_to_v3_tensor(self, v2_component: Dict[str, Any]) -> str:
        """Конвертация компонента v2 в тензорный формат v3"""
        comp_id = v2_component.get('id', 'unknown')
        comp_type = v2_component.get('t', 'component')[:20]  # Ограничиваем длину
        
        # Используем обычный D для совместимости
        symbol = 'D'
        
        # Строим тензорное выражение
        parts = []
        
        # Добавляем матрицу готовности (используем ASCII версии)
        if 'm' in v2_component:
            m = v2_component['m']
            parts.append(f"grad[{m.get('cur', 0)}->{m.get('tgt', 100)}]")
        
        # Добавляем зависимости
        if 'd' in v2_component:
            deps = v2_component['d']
            if isinstance(deps, list):
                parts.append(f"compose[{','.join(deps)}]")
        
        # Добавляем флаги как операторы
        if 'f' in v2_component:
            flags = v2_component['f']
            if 'async' in flags:
                parts.append('parallel[async]')
            if 'gpu' in flags:
                parts.append('partial[gpu]')
        
        operations = ' '.join(parts) if parts else 'grad[0->100]'
        
        return f"{symbol}[{comp_id}:{comp_type}] := {{{operations}}}"
    
    def validate_and_enhance(self, component: Dict[str, Any]) -> None:
        """Валидация и улучшение компонента"""
        # Добавляем вычисляемые метрики
        if 'm' in component:
            m = component['m']
            progress = (m['cur'] / m['tgt']) * 100 if m['tgt'] > 0 else 0
            component['m']['progress'] = round(progress, 2)
            
            # Определяем статус
            if progress >= 95:
                component['m']['status'] = 'complete'
            elif progress >= 75:
                component['m']['status'] = 'good'
            elif progress >= 50:
                component['m']['status'] = 'in_progress'
            else:
                component['m']['status'] = 'needs_work'
        
        # Добавляем категорию на основе флагов
        if 'f' in component:
            flags = component['f']
            if 'gpu' in flags or 'simd' in flags:
                component['category'] = 'performance'
            elif 'ai' in flags:
                component['category'] = 'ai_ml'
            elif 'async' in flags or 'streaming' in flags:
                component['category'] = 'async_io'
            else:
                component['category'] = 'core'
        
        # Валидация ID
        if not re.match(r'^[a-z0-9_]{1,64}$', component['id']):
            component['validation_warnings'] = component.get('validation_warnings', [])
            component['validation_warnings'].append(f"ID '{component['id']}' doesn't match pattern")
    
    def infer_kind(self, component_type: str, symbol: str = 'D') -> str:
        """Интеллектуальное определение типа компонента"""
        type_lower = component_type.lower()
        
        # Маппинг типов
        kind_map = {
            'test': 'T',
            'agent': 'A',
            'batch': 'B',
            'function': 'F',
            'func': 'F',
            'module': 'M',
            'mod': 'M',
            'service': 'S',
            'svc': 'S',
            'resource': 'R',
            'res': 'R',
            'process': 'P',
            'proc': 'P',
            'data': 'D',
            'error': 'E',
            'err': 'E',
            'component': 'C',
            'comp': 'C'
        }
        
        # Проверяем прямое соответствие
        for keyword, kind in kind_map.items():
            if keyword in type_lower:
                return kind
        
        # Используем символ как подсказку
        if symbol in ['D', 'Ⱦ']:
            return 'C'
        
        return 'C'  # По умолчанию компонент