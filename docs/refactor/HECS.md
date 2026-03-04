# HECS

## HECS architecture

Hierarchical ECS describes a novel compiler architecure based on the gamedev concept of an Entity Component System. Entities correspond to declarations, which come from the Syntax Tree.

## Entity

Entities have a kind describing what declaration they are, an id, and a syntax tree node.

## Components

Components describe the syntax, extracted from the syntax tree, for an entity. Each component describes an aspect of the entities behavior. Components are pure, derived entirely from the entities syntax tree.

Components include:

StaticSymbol
StaticSubscriptSymbol
InstanceSymbol
InstanceSubscriptSymbol
GenericParams
Callable
Documentation
Attributes
Typed
Valued
InstanceValued
InstanceTyped

## Systems

Systems are used to query the hECS. Systems are essentially a query based on components something has, or what its parent is.

## Queries

Everything within the hECS is entirely pure based on the local definition. In order to process on top of this, queries are used.

Queries are able to call each other, through which the query system will build up a dependency tree. Queries can also depend on systems, such that when a dependency changes, they will be invalidated.

For example, you might have a query that takes a generic parameter, and returns all constraints on it across everything.

Or there might be a query for a builtin, which will find the item with the builtin, and return it.