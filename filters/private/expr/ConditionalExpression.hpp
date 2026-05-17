#pragma once

#include "ConditionalParser.hpp"
#include "Expression.hpp"
#include "Lexer.hpp"

namespace pdal
{
namespace expr
{

class ConditionalExpression : public Expression
{
public:
    Utils::StatusWithReason prepare(PointLayoutPtr layout) override;
    bool eval(PointRef& p) const;
};

} // namespace expr

namespace Utils
{

template <>
inline StatusWithReason fromString(const std::string& from,
                                   pdal::expr::ConditionalExpression& expr)
{
    std::string error;

    expr::Lexer lexer(from);
    expr::ConditionalParser parser(lexer);
    if (!parser.expression(expr))
        return {-1, parser.error()};
    if (!parser.checkEnd())
        return {-1, "Found '" + from.substr(lexer.pos()) +
                        "' following valid expression."};
    return {};
}

} // namespace Utils

} // namespace pdal
